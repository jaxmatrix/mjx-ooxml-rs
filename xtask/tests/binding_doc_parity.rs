//! **A Python reader and a TypeScript reader are told the same thing about the same method**
//! (MJXOFF-266, the half MJXOFF-234 could not close).
//!
//! # The hole this closes
//!
//! MJXOFF-234 gave Python one source of prose: PyO3 compiles each `///` comment in
//! `bindings/mjx-python/src/` verbatim into the member's `__doc__`, `bindings/mjx-python/tools/stub_docs.py`
//! copies that into the committed `.pyi`, and `bindings/mjx-python/tests/test_stub_docs.py` is the
//! drift check. The TypeScript side already had one: `wasm-bindgen` generates `mjx_ooxml.d.ts` from
//! the `///` comments in `bindings/mjx-wasm/src/`.
//!
//! That left **two** sources and nothing between them. A sentence corrected in
//! `bindings/mjx-python/src/deck.rs` does not reach `bindings/mjx-wasm/src/deck.rs`, and H14 found
//! `refresh_chart_workbook` describing `regenerate_chart_workbook`'s behaviour in six places, three
//! per binding.
//!
//! **Read that last figure carefully, because it bounds what this file can claim.** Three per
//! binding means *both* bindings said the same wrong thing, and a parity gate would have been green
//! throughout. This file catches **asymmetric** drift — one side corrected, the other left — which
//! is the shape the next one takes now that the first has been fixed. It is not a truth check and
//! nothing here reads the facade.
//!
//! # Equality cannot be the gate, so the comparison is normalised first
//!
//! Much of what differs is *forced*: a sentence naming a sibling spells it `chart_series` in Python
//! and `chartSeries` in TypeScript, a value is `str` against `string` and `None` against
//! `undefined`. So [`normalised`] applies the same camel-case transform
//! `xtask/tests/binding_projection.rs` already enforces on names, inside backtick spans, plus a
//! closed table of language substitutions ([`SUBSTITUTIONS`]) and article removal. The transform is
//! what makes the residue small enough to be a ledger with a reason per row rather than a threshold:
//! when it was written it took the residue from 173 members to 47, and the twenty-three of *those*
//! that turned out to be defects rather than forced differences were fixed, leaving [`DIVERGENT`].
//! [`the_scanner_still_sees_both_bindings_prose`] prints today's counts, which is where they stay
//! true.
//!
//! # The pairing goes through the signature, which is what makes it sound
//!
//! MJXOFF-266 warns that matching on `(class, Rust name)` alone pairs `BorderEdgeSpec.new` in one
//! binding with a **different constructor** of the same name in the other — Python's takes a style
//! and a colour, JavaScript's takes nothing and the styles arrive through a builder chain. It does,
//! and the fix is not a ledger entry: **two members are a pair only when they also take the same
//! number of arguments.** Where the signatures differ the projection differs, and the prose is
//! *supposed* to differ; comparing it would be comparing two different methods.
//!
//! Every pair excluded that way when this was written was a shape `CLAUDE.md` already names as
//! forced — a range becoming two numbers, a `Vec<Vec<f64>>` becoming one series at a time, a list of
//! tuples becoming two parallel arrays — plus the two constructors above; the count is printed
//! rather than written here. PyO3's `Python<'_>` token is machinery rather than an argument of the
//! projected method and is not counted, which is what keeps thirty-three same-method pairs inside
//! the comparison instead of outside it.
//!
//! # What building this found
//!
//! Twenty-three members where the TypeScript reader was told **strictly less** than the Python one,
//! all fixed in the same commit: six `CellFormatSpec.applies*` getters that never said `undefined`
//! writes no attribute, `Document.open` with no error paragraph at all where `Workbook.open` beside
//! it had one, `FontProperties.scheme` documented as the single word "`scheme`", and the schema
//! attributes (`w:vanish`, `w:jc`, `w:outlineLvl`, `w:val="auto"`, `ST_HpsMeasure`) that the Word
//! effective-properties getters name in Python and named nowhere in TypeScript. The ticket assumed
//! the residue was mostly forced; roughly half of it was not.
//!
//! # Every arm was made to fail
//!
//! | Mutation | What failed |
//! |---|---|
//! | one word changed in `bindings/mjx-wasm/src/deck.rs`'s `add_slide` comment | [`every_member_both_bindings_project_is_described_the_same_way`], naming the member and printing both sentences |
//! | a `DIVERGENT` row whose two sentences were then made to agree | [`every_divergent_row_still_names_a_pair_that_diverges`] |
//! | the `///` scanner's needle mistyped | [`the_scanner_still_sees_both_bindings_prose`] |

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ===============================================================================================
// The ledger
// ===============================================================================================

/// One member whose two sentences differ for a reason somebody wrote down.
struct Divergent {
    /// The class, or `<module>` for a free function.
    owner: &'static str,
    /// The Rust name both bindings spell the member with.
    member: &'static str,
    /// Why the two sentences cannot be the same one. A reader who disagrees has found a member that
    /// should have been unified rather than recorded.
    reason: &'static str,
}

/// Every member both bindings project under the same name and the same arity whose prose still
/// differs after [`normalised`].
///
/// Held to the measurement in both directions: a new divergence fails
/// [`every_member_both_bindings_project_is_described_the_same_way`] until somebody writes a reason,
/// and a row whose two sentences have since been unified fails
/// [`every_divergent_row_still_names_a_pair_that_diverges`] until somebody deletes it.
const DIVERGENT: &[Divergent] = &[
    Divergent {
        owner: "CellFormatSpec",
        member: "text_is_quote_prefixed",
        reason: "The TypeScript sentence carries a naming note with no Python counterpart: the \
                 readable getter is spelled as the facade's own field so both bindings read alike, \
                 while the builder beside it keeps the shorter `withQuotePrefix` it shipped with. \
                 Python renames nothing and has no such pair to explain.",
    },
    Divergent {
        owner: "ChartData",
        member: "validate",
        reason: "The two error models. Python raises a typed exception class \
                 (`InvalidArgumentError`); JavaScript throws one `OoxmlError` carrying a `code`. \
                 Neither sentence can be written in the other language's terms without being wrong.",
    },
    Divergent {
        owner: "ChartSeriesFreshnessInfo",
        member: "from_cells",
        reason: "Both explain why `clippy::wrong_self_convention` is allowed here, and each names \
                 its own reader — a third spelling of one field in front of a *Python* caller \
                 against a *TypeScript* one — with Python additionally citing the crate's \
                 renames-nothing rule, which has no wasm counterpart.",
    },
    Divergent {
        owner: "ChartWrap",
        member: "kind",
        reason: "Not a prose difference at all: the two bindings return **different strings** for \
                 the same wrap — `\"top_and_bottom\"` in Python, `\"topAndBottom\"` in JavaScript — \
                 and each sentence correctly names its own. That is a value divergence in a data \
                 token, and `ShapeGeometry.of`'s own wasm comment argues data keys stay \
                 `snake_case`; MJXOFF-268 owns it. This row must be deleted, not edited, when it is.",
    },
    Divergent {
        owner: "CustomGeometrySpec",
        member: "guide_values",
        reason: "The TypeScript sentence names the return shape (`a record from guide name to \
                 number`) because a `.d.ts` reader has no other statement of it; the Python stub \
                 carries the annotation in the signature. Plus the two error models.",
    },
    Divergent {
        owner: "Deck",
        member: "blank",
        reason: "Each sentence names its own reason for authoring every part rather than embedding \
                 a template — buildable from a `pip install` with no input file, against buildable \
                 in a browser with no input file. Plus the two error models.",
    },
    Divergent {
        owner: "Deck",
        member: "open",
        reason: "Python's sentence documents releasing the interpreter lock, which has no \
                 JavaScript counterpart; the wasm sentence shows the browser call \
                 (`new Uint8Array(await file.arrayBuffer())`), which has no Python counterpart. \
                 Plus the two error models.",
    },
    Divergent {
        owner: "Deck",
        member: "save",
        reason: "The same pair as `open`: the interpreter lock against the `Blob` a browser caller \
                 wraps the bytes in, and the two error models.",
    },
    Divergent {
        owner: "Deck",
        member: "validate",
        reason: "The two error models, and nothing else: Python names a typed exception class, \
                 JavaScript one `OoxmlError` carrying a `code`. The sentence describing what is \
                 checked is word for word the same in both.",
    },
    Divergent {
        owner: "Document",
        member: "cell_span",
        reason: "Python returns a tuple and its sentence says so (`as (rows, columns)`); \
                 wasm-bindgen cannot project a tuple, so JavaScript gets an exported object whose \
                 own getters name the two numbers and the sentence has nothing to add.",
    },
    Divergent {
        owner: "Document",
        member: "open",
        reason: "Python's sentence documents releasing the interpreter lock. Plus the two error \
                 models.",
    },
    Divergent {
        owner: "Document",
        member: "save",
        reason: "Python's sentence documents releasing the interpreter lock for the write.",
    },
    Divergent {
        owner: "Document",
        member: "set_section_page_margins",
        reason: "The TypeScript sentence names the value that removes them (`given undefined`) \
                 because a `.d.ts` optional argument does not say what passing nothing means; the \
                 Python stub's `| None` annotation does.",
    },
    Divergent {
        owner: "Document",
        member: "set_section_page_size",
        reason: "The same as `set_section_page_margins` beside it, for the same reason.",
    },
    Divergent {
        owner: "Document",
        member: "table_dimensions",
        reason: "The same tuple-against-object difference as `cell_span`.",
    },
    Divergent {
        owner: "ErrorBarSpec",
        member: "validate",
        reason: "The two error models, and nothing else: Python names a typed exception class, \
                 JavaScript one `OoxmlError` carrying a `code`. The sentence describing what is \
                 checked is word for word the same in both.",
    },
    Divergent {
        owner: "IndentLevel",
        member: "new",
        reason: "Python raises `ValueError`, which is the interpreter's own class for an argument \
                 out of range rather than one of this binding's; JavaScript throws. There is no \
                 shared spelling.",
    },
    Divergent {
        owner: "ShapeGeometry",
        member: "adjustments",
        reason: "The TypeScript sentence names the return shape (`a record from name to Fraction or \
                 Angle`), which the Python stub states in its annotation instead.",
    },
    Divergent {
        owner: "ShapeGeometry",
        member: "of",
        reason: "Each carries a worked example **in its own language** — a fenced `python` block \
                 against a fenced `js` one — and the wasm sentence additionally argues why the \
                 adjustment keys stay `snake_case` where every method name is camel-cased, which is \
                 a question Python does not have.",
    },
    Divergent {
        owner: "TrendlineSpec",
        member: "validate",
        reason: "The two error models, and nothing else: Python names a typed exception class, \
                 JavaScript one `OoxmlError` carrying a `code`. The sentence describing what is \
                 checked is word for word the same in both.",
    },
    Divergent {
        owner: "Workbook",
        member: "open",
        reason: "Python's sentence documents releasing the interpreter lock. Plus the two error \
                 models. The `.xlsb` refusal, which is the substance, is word for word the same in \
                 both.",
    },
    Divergent {
        owner: "Workbook",
        member: "read_range",
        reason: "Each points at its own guide for the measurements behind having no per-cell call \
                 beside this one — *The mapping rules* against *The TypeScript surface*. The two \
                 guides are different documents, so the citation cannot be shared.",
    },
    Divergent {
        owner: "Workbook",
        member: "save",
        reason: "Python's sentence documents releasing the interpreter lock for the write.",
    },
    Divergent {
        owner: "Workbook",
        member: "write_cells",
        reason: "The TypeScript sentence carries a warning with no Python counterpart: an array of \
                 exported objects crosses the wasm boundary **by value**, so every `CellWrite` in \
                 the batch is consumed and must not be freed. Python's objects are reference \
                 counted and nothing is consumed.",
    },
];

// ===============================================================================================
// The scanner
// ===============================================================================================

/// The repository root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// One documented member of one binding.
#[derive(Clone)]
struct Documented {
    /// The `///` comment, every line joined with one space.
    prose: String,
    /// How many arguments the projected method takes, `self` and PyO3's `Python<'_>` token
    /// excluded — see the module docs for why this is part of the key.
    arity: usize,
}

/// Every `///`-documented `fn` inside an `impl` block annotated with `marker`, keyed by
/// `(owner, Rust name)`.
///
/// Deliberately over `src/` rather than over the generated `.d.ts` or the committed `.pyi`: the
/// `.d.ts` is build output and git-ignored, and the `.pyi`'s docstrings are themselves generated
/// from these comments (MJXOFF-234). The `///` comments *are* both surfaces' prose.
fn documented_members(directory: &Path, marker: &str) -> BTreeMap<(String, String), Documented> {
    let mut found = BTreeMap::new();
    for (_, text) in xtask::binding_surface::sources(directory, "rs") {
        let lines: Vec<&str> = text.lines().collect();
        let mut index = 0;
        while index < lines.len() {
            let Some(owner) = impl_target(lines[index]) else {
                index += 1;
                continue;
            };
            if !lines[index.saturating_sub(6)..index]
                .iter()
                .any(|line| line.trim() == marker)
            {
                index += 1;
                continue;
            }
            let end = block_end(&lines, index);
            let mut prose: Vec<String> = Vec::new();
            let mut at = index + 1;
            while at <= end {
                let line = lines[at].trim();
                if let Some(rest) = line.strip_prefix("///") {
                    prose.push(rest.trim().to_owned());
                } else if let Some(name) = function_name(line) {
                    if !prose.is_empty() {
                        let signature = signature(&lines, at, end);
                        found.insert(
                            (owner.clone(), name),
                            Documented {
                                prose: prose.join(" ").trim().to_owned(),
                                arity: arity(&signature),
                            },
                        );
                    }
                    prose.clear();
                } else if !line.is_empty() && !line.starts_with("#[") && !line.starts_with("//") {
                    prose.clear();
                }
                at += 1;
            }
            index = end + 1;
        }
    }
    found
}

/// `impl Foo {` — the type an impl block is written for, or `None` for anything else.
fn impl_target(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("impl ")?;
    let name: String = rest
        .chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect();
    let tail = rest[name.len()..].trim();
    (!name.is_empty() && (tail == "{" || tail.is_empty())).then_some(name)
}

/// The index of the line closing the block opened at `from`.
fn block_end(lines: &[&str], from: usize) -> usize {
    let mut depth = 0i32;
    for (offset, line) in lines[from..].iter().enumerate() {
        depth += i32::try_from(line.matches('{').count()).unwrap_or(0);
        depth -= i32::try_from(line.matches('}').count()).unwrap_or(0);
        if depth == 0 && offset > 0 {
            return from + offset;
        }
    }
    lines.len() - 1
}

/// `pub fn name(` / `fn name(` — the Rust name, or `None` for anything else.
///
/// The `<` is not optional decoration: `bindings/mjx-python/src/deck.rs` writes
/// `fn save<'py>(&self, python: Python<'py>)`, and a needle that demands `(` immediately after the
/// name misses every `PyBytes`-returning method in the crate — which is `save`, `save_unchecked`
/// and every `*_bytes` reader, the members most worth comparing.
fn function_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("pub ").unwrap_or(line);
    let rest = rest.strip_prefix("fn ")?;
    let name: String = rest
        .chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect();
    let after = &rest[name.len()..];
    (!name.is_empty() && (after.starts_with('(') || after.starts_with('<'))).then_some(name)
}

/// The text between the parentheses of the signature beginning on line `at`, however many lines it
/// is wrapped across.
fn signature(lines: &[&str], at: usize, end: usize) -> String {
    let mut text = String::new();
    for line in &lines[at..=end] {
        text.push_str(line.trim());
        text.push(' ');
        if text.matches('(').count() > 0 && text.matches('(').count() == text.matches(')').count() {
            break;
        }
    }
    let Some(open) = text.find('(') else {
        return String::new();
    };
    let mut depth = 0usize;
    for (offset, character) in text[open..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return text[open + 1..open + offset].to_owned();
                }
            }
            _ => {}
        }
    }
    String::new()
}

/// How many arguments a signature takes, excluding `self` and PyO3's `Python<'_>` token.
///
/// The token is machinery rather than an argument of the projected method: counting it would put
/// `Deck.open` at two against JavaScript's one and exclude thirty-three pairs that are the same
/// method.
fn arity(signature: &str) -> usize {
    let mut depth = 0i32;
    let mut arguments = Vec::new();
    let mut current = String::new();
    for character in signature.chars() {
        match character {
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => depth -= 1,
            ',' if depth == 0 => {
                arguments.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(character);
    }
    arguments.push(current);
    arguments
        .iter()
        .map(|argument| argument.trim())
        .filter(|argument| !argument.is_empty())
        .filter(|argument| {
            let bare = argument.trim_start_matches('&').trim_start();
            let bare = bare.strip_prefix("mut ").unwrap_or(bare);
            !bare.starts_with("self")
        })
        .filter(|argument| !argument.contains("Python<") && !argument.ends_with(": Python"))
        .count()
}

// ===============================================================================================
// The transform
// ===============================================================================================

/// The language substitutions that are not identifier case — applied outside and inside code spans
/// alike, because `str` against `string` occurs both ways.
///
/// A closed table on purpose. Adding a row here weakens every comparison in the file, so a row is
/// added only for a word pair the two languages genuinely force, never to quiet one member.
const SUBSTITUTIONS: [(&str, &str); 15] = [
    ("string", "\u{1}str"),
    ("str", "\u{1}str"),
    ("null", "\u{1}none"),
    ("none", "\u{1}none"),
    ("undefined", "\u{1}none"),
    ("array", "\u{1}list"),
    ("list", "\u{1}list"),
    ("number", "\u{1}float"),
    ("float", "\u{1}float"),
    ("boolean", "\u{1}bool"),
    ("bool", "\u{1}bool"),
    ("javascript", "\u{1}lang"),
    ("typescript", "\u{1}lang"),
    ("python", "\u{1}lang"),
    ("raises", "\u{1}raise"),
];

/// `text` reduced to what the two languages must say alike.
///
/// Three steps, in order: every identifier inside a backtick span is written in `snake_case` (the
/// inverse of the camel-case rule `xtask/tests/binding_projection.rs` enforces on names); the words
/// in [`SUBSTITUTIONS`] are folded onto one spelling; and articles and whitespace are dropped, which
/// is what lets *"as `str`"* and *"as a `string`"* compare equal.
fn normalised(text: &str) -> String {
    // Case-folding comes **after** the span transform, not before: `seriesIdx` lower-cased first is
    // `seriesidx`, which has no capital left to split on and pairs with nothing.
    let mut spanned = String::with_capacity(text.len());
    let mut in_span = false;
    for chunk in text.split('`') {
        if in_span {
            spanned.push('`');
            spanned.push_str(&snake_cased_identifiers(chunk));
            spanned.push('`');
        } else {
            spanned.push_str(chunk);
        }
        in_span = !in_span;
    }
    let out = spanned.to_lowercase();
    let mut words: Vec<String> = Vec::new();
    for word in out.split_whitespace() {
        let trimmed: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
        if matches!(trimmed.as_str(), "a" | "an" | "the") {
            continue;
        }
        let mut word = word.to_owned();
        for (from, to) in SUBSTITUTIONS {
            word = replace_word(&word, from, to);
        }
        words.push(word.replace('\u{1}', ""));
    }
    words.join(" ")
}

/// `text` with every whole-word occurrence of `from` replaced by `to`.
///
/// Whole-word so that `boolean` inside `booleans` is left alone, and so that a substitution cannot
/// eat a longer identifier it happens to prefix.
fn replace_word(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(from) {
        let before_is_word = rest[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
        let after = &rest[at + from.len()..];
        let after_is_word = after
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
        out.push_str(&rest[..at]);
        if before_is_word || after_is_word {
            out.push_str(from);
        } else {
            out.push_str(to);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Every dotted identifier in a code span written in `snake_case`.
///
/// Dotted, so `Workbook.chartSeries` becomes `workbook.chart_series` and pairs with Python's
/// `Workbook.chart_series`; segment by segment, so a `.` never becomes a `_`.
fn snake_cased_identifiers(span: &str) -> String {
    span.split_whitespace()
        .map(|word| {
            if word
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
                && word.starts_with(|c: char| c.is_ascii_alphabetic())
            {
                word.split('.')
                    .map(snake_case)
                    .collect::<Vec<_>>()
                    .join(".")
            } else {
                word.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `chartSeries` -> `chart_series`, and `chart_series` unchanged.
fn snake_case(word: &str) -> String {
    let mut out = String::with_capacity(word.len() + 4);
    for (index, character) in word.char_indices() {
        if character.is_ascii_uppercase() && index > 0 {
            out.push('_');
        }
        out.push(character.to_ascii_lowercase());
    }
    out
}

// ===============================================================================================
// The anti-vacuity floors
// ===============================================================================================

/// Each binding must still be seen to document a surface at all.
const MINIMUM_DOCUMENTED_MEMBERS: usize = 1_400;

/// …and the pairing must still be pairing. This is the floor that matters: a scanner that reads
/// both trees and pairs nothing reports *no member is described differently* over an empty
/// comparison, which is the clean bill this file exists not to hand out. Phrased as *the scanner has
/// stopped matching*, never as the exact count, so a genuine divergence fails by name rather than
/// being pre-empted here.
const MINIMUM_COMPARED_PAIRS: usize = 1_300;

// ===============================================================================================
// The tests
// ===============================================================================================

/// Every member of the two surfaces both bindings project under the same name and the same arity.
fn compared_pairs() -> Vec<(String, String, Documented, Documented)> {
    let root = repository_root();
    let python = documented_members(&root.join("bindings/mjx-python/src"), "#[pymethods]");
    let wasm = documented_members(&root.join("bindings/mjx-wasm/src"), "#[wasm_bindgen]");
    python
        .into_iter()
        .filter_map(|((owner, member), py)| {
            let ws = wasm.get(&(owner.clone(), member.clone()))?;
            (py.arity == ws.arity).then(|| (owner, member, py, ws.clone()))
        })
        .collect()
}

/// Both scanners are alive and the pairing is pairing.
#[test]
fn the_scanner_still_sees_both_bindings_prose() {
    let root = repository_root();
    let python = documented_members(&root.join("bindings/mjx-python/src"), "#[pymethods]");
    let wasm = documented_members(&root.join("bindings/mjx-wasm/src"), "#[wasm_bindgen]");
    let pairs = compared_pairs();
    println!(
        "binding doc parity: {} documented Python members, {} documented wasm members, {} compared \
         pairs ({} excluded because the two signatures take different numbers of arguments)",
        python.len(),
        wasm.len(),
        pairs.len(),
        python
            .keys()
            .filter(|key| wasm.contains_key(*key))
            .count()
            - pairs.len(),
    );
    assert!(
        python.len() >= MINIMUM_DOCUMENTED_MEMBERS,
        "only {} documented Python member(s) found — the `///` scanner has stopped matching",
        python.len()
    );
    assert!(
        wasm.len() >= MINIMUM_DOCUMENTED_MEMBERS,
        "only {} documented wasm member(s) found — the `///` scanner has stopped matching",
        wasm.len()
    );
    assert!(
        pairs.len() >= MINIMUM_COMPARED_PAIRS,
        "only {} pair(s) compared — the pairing has stopped matching, and every claim below would \
         be made over almost nothing",
        pairs.len()
    );
}

/// The gate itself.
#[test]
fn every_member_both_bindings_project_is_described_the_same_way() {
    let ledger: BTreeSet<(&str, &str)> = DIVERGENT
        .iter()
        .map(|row| (row.owner, row.member))
        .collect();
    let pairs = compared_pairs();
    let mut unexplained = Vec::new();
    let mut agreeing = 0usize;
    for (owner, member, python, wasm) in &pairs {
        if normalised(&python.prose) == normalised(&wasm.prose) {
            agreeing += 1;
            continue;
        }
        if !ledger.contains(&(owner.as_str(), member.as_str())) {
            unexplained.push(format!(
                "{owner}.{member}\n      Python: {}\n        wasm: {}",
                python.prose, wasm.prose
            ));
        }
    }
    println!(
        "binding doc parity: {agreeing} of {} pairs describe the member the same way; {} on the \
         ledger",
        pairs.len(),
        ledger.len()
    );
    assert!(
        unexplained.is_empty(),
        "these members are projected by both bindings under the same name and the same signature, \
         and the two sentences say different things. Unify them — or, if the difference is forced \
         by the two languages, add a row to `DIVERGENT` saying so:\n    {}",
        unexplained.join("\n    ")
    );
    assert!(
        agreeing >= MINIMUM_COMPARED_PAIRS,
        "only {agreeing} pair(s) agree — the comparison has stopped matching"
    );
}

/// …and no row survives the divergence it was written about.
#[test]
fn every_divergent_row_still_names_a_pair_that_diverges() {
    let pairs = compared_pairs();
    let mut stale = Vec::new();
    for row in DIVERGENT {
        let found = pairs
            .iter()
            .find(|(owner, member, _, _)| owner == row.owner && member == row.member);
        match found {
            None => stale.push(format!(
                "{}.{} is no longer a member both bindings project under one name and one signature",
                row.owner, row.member
            )),
            Some((_, _, python, wasm)) if normalised(&python.prose) == normalised(&wasm.prose) => {
                stale.push(format!(
                    "{}.{} now says the same thing in both bindings — delete the row",
                    row.owner, row.member
                ));
            }
            Some(_) => {}
        }
        assert!(
            row.reason.len() > 40,
            "{}.{} has no reason worth reading",
            row.owner,
            row.member
        );
    }
    assert!(
        stale.is_empty(),
        "these `DIVERGENT` rows no longer name a divergence:\n  {}",
        stale.join("\n  ")
    );
}
