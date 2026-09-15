//! The deferral gate (MJXOFF-298): a comment in a workspace member's `src/` that defers a feature cites the ticket that owns it, or is recorded in `docs/client-platform/data/unowned_deferrals.json`.

#[path = "../src/json.rs"]
mod json;
#[path = "../src/ticket_roster.rs"]
mod ticket_roster;

use json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use ticket_roster::{closed_reason, is_open_ticket};
use xtask::repository_files::WorkingTree;

// How a phrase's occurrence becomes a deferral.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Rule {
    // Every occurrence is one.
    Anywhere,
    // Only where its sentence waits on work, or `until then` in a paragraph that names that work.
    UntilWorkArrives,
    // Only in a sentence that also carries a TEMPORARY_CUES word.
    InTemporarySentence,
    // Only in a paragraph that also carries a BLOCKING_CUES phrase.
    InBlockedParagraph,
}

struct Phrase {
    spelling: &'static str,
    case_sensitive: bool,
    rule: Rule,
    reason: &'static str,
}

// A case-insensitive phrase read under `rule`.
const fn phrase(spelling: &'static str, rule: Rule, reason: &'static str) -> Phrase {
    Phrase {
        spelling,
        case_sensitive: false,
        rule,
        reason,
    }
}

// The deferral phrases, each with why it marks a deferral and how narrowly it is read.
const PHRASES: [Phrase; 16] = [
    phrase("stated gap", Rule::Anywhere, "admits behaviour the code knows it lacks"),
    phrase(
        "stated limitation",
        Rule::Anywhere,
        "loop 1's second spelling of a stated gap, in the same sentence shape",
    ),
    phrase(
        "not built yet",
        Rule::Anywhere,
        "says the missing piece is still to be written",
    ),
    phrase("is owed to", Rule::Anywhere, "hands the missing piece to later work"),
    phrase(
        "not yet",
        Rule::Anywhere,
        "says a capability is still to come; a value in flight is a scoped exception",
    ),
    phrase(
        "not drawn",
        Rule::Anywhere,
        "admits a feature the file carries reaches no pixel",
    ),
    phrase(
        "not implemented",
        Rule::Anywhere,
        "admits behaviour the code knows it lacks",
    ),
    phrase("future task", Rule::Anywhere, "hands the missing piece to later work"),
    phrase("later unit", Rule::Anywhere, "hands the missing piece to later work"),
    phrase("for now", Rule::Anywhere, "says the present behaviour is temporary"),
    phrase(
        "wait for",
        Rule::Anywhere,
        "says a feature is held back on something not yet present",
    ),
    phrase(
        "not honoured",
        Rule::Anywhere,
        "admits an attribute the file carries is ignored",
    ),
    phrase(
        "until",
        Rule::UntilWorkArrives,
        "a deferral only when what it waits on is work, not a value or an event at run time",
    ),
    phrase(
        "deliberately not",
        Rule::InTemporarySentence,
        "a design decision, and a deferral only when its sentence says it is temporary",
    ),
    phrase(
        "stand-in",
        Rule::InTemporarySentence,
        "the placeholder-geometry vocabulary, and a deferral only when its sentence says it is temporary",
    ),
    Phrase {
        spelling: "GUESS",
        case_sensitive: true,
        rule: Rule::InBlockedParagraph,
        reason: "an unverified reading of Office, and a deferral only when its paragraph says the real answer is blocked",
    },
];

// Sentence words that make an `until` wait on work rather than on a value; a ticket number alone is usually history.
const UNTIL_WORK_CUES: [&str; 8] = [
    "exists",
    "lands",
    "is built",
    "is implemented",
    "is written",
    "arrives",
    "grows",
    "ticket",
];

// Sentence words that make a decision or a placeholder temporary.
const TEMPORARY_CUES: [&str; 4] = ["yet", "until", "for now", "for the moment"];

// Paragraph phrases that say a GUESS stands in for an answer something missing blocks.
const BLOCKING_CUES: [&str; 18] = [
    "yet",
    "until",
    "not implemented",
    "reads no",
    "reads none",
    "needs the",
    "cannot express",
    "does not honour",
    "not honoured",
    "does not read",
    "does not carry",
    "does not have",
    "does not make",
    "does not model",
    "resolves none",
    "needs a",
    "has no cell",
    "is a change to",
];

// Occurrences that match a phrase and defer nothing: path, phrase, an excerpt of the matched text, and why.
const NOT_DEFERRALS: &[(&str, &str, &str, &str)] = &[
    (
        "crates/mjx-docx/src/document/mod.rs",
        "not yet",
        "a `styles.xml` a document does not yet have",
        "a document lacking a part at run time, which this method creates",
    ),
    (
        "crates/mjx-xlsx/src/blank.rs",
        "not yet",
        "a part a workbook does not yet have",
        "a workbook lacking a part at run time, which this module creates",
    ),
    (
        "crates/mjx-opc/src/package.rs",
        "not yet",
        "so it is not yet a",
        "an empty package before a format layer builds on it",
    ),
    (
        "crates/mjx-docx/src/lib.rs",
        "not yet",
        "everything it does not yet model is preserved verbatim",
        "the round-trip contract's general statement, naming no feature",
    ),
    (
        "crates/mjx-docx/src/document/table_properties.rs",
        "not yet",
        "(`tables.rs`) does not yet type",
        "names the members this module types, which `tables.rs` left opaque",
    ),
    (
        "crates/mjx-pptx/src/presentation/text.rs",
        "not yet",
        "not yet typed into",
        "a placeholder nobody has typed into, which is document state",
    ),
    (
        "crates/mjx-allocation-counter/src/lib.rs",
        "not yet",
        "allocated and not yet freed",
        "allocator bytes still live, which is run-time state",
    ),
    (
        "crates/mjx-canvas-harness/src/canvas.rs",
        "not yet",
        "not yet turned into a fragment",
        "a node awaiting its fragment within one pass",
    ),
    (
        "crates/mjx-canvas-harness/src/scenes/feedback.rs",
        "not yet",
        "content not yet laid out",
        "names the placeholder element shown for content awaiting layout",
    ),
    (
        "crates/mjx-dml/src/geometry/formula.rs",
        "not yet",
        "has not yet been calculated",
        "quotes ECMA-376 on a guide's evaluation order",
    ),
    (
        "crates/mjx-docx/src/document/residency.rs",
        "not yet",
        "read but not yet resolved",
        "a part read and awaiting resolution within one pass",
    ),
    (
        "crates/mjx-layout-docx/src/checkpoint.rs",
        "not yet",
        "not yet placed",
        "a checkpoint field naming the note lines a page still has to place",
    ),
    (
        "crates/mjx-layout-docx/src/flow.rs",
        "not yet",
        "laid out but not yet placed on a page",
        "a line awaiting its page within one pass",
    ),
    (
        "crates/mjx-layout-docx/src/model.rs",
        "not yet",
        "laid out but not yet turned into fragments",
        "a page awaiting its fragments within one pass",
    ),
    (
        "crates/mjx-layout-pptx/src/autofit.rs",
        "not yet",
        "has not yet had to scale",
        "a text body autofit has not needed to scale, which is document state",
    ),
    (
        "crates/mjx-layout-pptx/src/body.rs",
        "not yet",
        "composed but not yet given a position",
        "a composed line awaiting its position within one pass",
    ),
    (
        "crates/mjx-opc/src/validate.rs",
        "not yet",
        "Bytes not yet parsed",
        "part bytes awaiting tokenisation",
    ),
    (
        "crates/mjx-session/src/journal.rs",
        "not yet",
        "first entry not yet handed to the sink",
        "journal records not yet written to the sink, which is run-time state",
    ),
    (
        "crates/mjx-session/src/journal.rs",
        "not yet",
        "have not yet been handed to the sink",
        "journal records not yet written to the sink, which is run-time state",
    ),
    (
        "crates/mjx-session/src/journal.rs",
        "not yet",
        "everything not yet flushed",
        "journal records not yet written to the sink, which is run-time state",
    ),
    (
        "crates/mjx-session/src/session.rs",
        "not yet",
        "everything not yet handed to the journal sink",
        "journal records not yet written to the sink, which is run-time state",
    ),
    (
        "xtask/src/repository_files.rs",
        "not yet",
        "not yet committed it",
        "a working-tree file awaiting its commit",
    ),
    (
        "crates/mjx-layout-chart/src/geometry.rs",
        "not drawn",
        "a point that is not drawn rather than a point that is not there",
        "a blank point draws nothing by definition",
    ),
    (
        "crates/mjx-docx/src/document/residency.rs",
        "not drawn",
        "its number was not drawn at all",
        "retrospective: the list number this module now resolves",
    ),
    (
        "crates/mjx-layout-pptx/src/model.rs",
        "not drawn",
        "an edge that is not drawn as a band",
        "describes the `None` a diagonal edge returns, whose owner the diagonal paragraph cites",
    ),
    (
        "crates/mjx-layout-xlsx/src/border.rs",
        "not drawn",
        "because it is not drawn at all",
        "a `none` border draws nothing by definition",
    ),
    (
        "crates/mjx-layout-xlsx/src/border.rs",
        "not drawn",
        "Not drawn.",
        "the `none` arm of the same table",
    ),
    (
        "crates/mjx-layout-xlsx/src/border.rs",
        "not drawn",
        "becomes the one that is not drawn",
        "the rounding hazard the band table is written to avoid",
    ),
    (
        "crates/mjx-paint/src/backend/execute.rs",
        "not drawn",
        "why the frame is not drawn into directly",
        "a render-path design choice",
    ),
    (
        "crates/mjx-paint/src/export/pdf.rs",
        "not drawn",
        "Added to the clipping path and not drawn",
        "PDF clip semantics",
    ),
    (
        "crates/mjx-paint/src/pattern.rs",
        "not drawn",
        "derived, not drawn",
        "the masks are computed rather than hand-drawn",
    ),
    (
        "crates/mjx-scene-xlsx/src/colour.rs",
        "not drawn",
        "a border that is not drawn and a font that is not there",
        "the defect this conversion prevents",
    ),
    (
        "xtask/src/ledger/assess.rs",
        "not drawn",
        "evidence is not drawn",
        "the ledger's own state vocabulary",
    ),
    (
        "crates/mjx-layout-xlsx/src/condfmt/mod.rs",
        "not implemented",
        "indistinguishable from one that is not implemented",
        "why a rule's firing is reported",
    ),
    (
        "crates/mjx-layout-xlsx/src/model.rs",
        "not implemented",
        "a rule that is not implemented render identically",
        "why a rule's firing is reported",
    ),
    (
        "crates/mjx-layout-docx/src/model.rs",
        "wait for",
        "cannot afford to wait for",
        "why a page count is estimated",
    ),
    (
        "crates/mjx-paint/src/backend/device.rs",
        "wait for",
        "Wait for everything submitted so far",
        "GPU queue synchronisation",
    ),
    (
        "crates/mjx-session/src/document.rs",
        "wait for",
        "has to wait for a clock",
        "a commit trigger's purpose",
    ),
    (
        "crates/mjx-session/src/journal.rs",
        "wait for",
        "A crash does not wait for a record",
        "the crash-safety framing of the record format",
    ),
    (
        "crates/mjx-session/src/ooxml/mod.rs",
        "wait for",
        "does not have to wait for a clock",
        "a commit trigger's purpose",
    ),
    (
        "crates/mjx-session/src/schedule.rs",
        "wait for",
        "should not wait for a clock",
        "a commit trigger's purpose",
    ),
    (
        "crates/mjx-session/src/schedule.rs",
        "wait for",
        "wait for the gesture to end",
        "the scheduler's gesture policy",
    ),
    (
        "crates/mjx-session/src/schedule.rs",
        "wait for",
        "now wait for it to end",
        "the scheduler's gesture policy",
    ),
    (
        "crates/mjx-session/src/session.rs",
        "wait for",
        "triggers now wait for it to end",
        "the scheduler's gesture policy",
    ),
    (
        "crates/mjx-view/src/viewport.rs",
        "wait for",
        "must not wait for a re-layout",
        "the prefetch ring's rationale",
    ),
    (
        "crates/mjx-docx/src/document/charts.rs",
        "not honoured",
        "written as the element the schema requires but not honoured",
        "`simplePos=\"0\"` is written by design so the stated position is used",
    ),
    (
        "crates/mjx-scene/src/tessellate.rs",
        "not honoured",
        "is clamped, not honoured",
        "the tolerance floor is permanent policy",
    ),
    (
        "crates/mjx-paint/src/export/pdf.rs",
        "until",
        "not complete until the page's last run is written",
        "a glyph set in flight within one export",
    ),
    (
        "crates/mjx-pptx/src/blank.rs",
        "until",
        "nothing renders it until a notes master exists",
        "a document gaining a notes master, which is document state",
    ),
    (
        "crates/mjx-pptx/src/blank.rs",
        "until",
        "Nothing renders it until a notes master exists",
        "a document gaining a notes master, which is document state",
    ),
    (
        "crates/mjx-ooxml/src/error.rs",
        "deliberately not",
        "deliberately not `#[non_exhaustive]`",
        "its `until` is the compiler refusing an unclassified variant, which is permanent",
    ),
    (
        "crates/mjx-scene/src/provider.rs",
        "stand-in",
        "the stand-in that answers until somebody real does",
        "the provider seam's run-time fall-back, which is permanent policy",
    ),
    (
        "crates/mjx-render-oracle/src/plate.rs",
        "stand-in",
        "Until Phase G",
        "retrospective: preset geometry landed in Phase G",
    ),
    (
        "crates/mjx-layout-docx/src/lib.rs",
        "GUESS",
        "marks each one `GUESS:`",
        "describes the crate's GUESS convention and marks no feature",
    ),
];

// The placeholder macros a library source may not hold without an owning ticket.
const PLACEHOLDER_MACROS: [&str; 2] = ["todo!(", "unimplemented!("];

// Floors that say the corpus and the comment reader still reach the tree, not its exact size.
const MINIMUM_FILES: usize = 300;
const MINIMUM_COMMENTS: usize = 5_000;

const UNOWNED: &str = "docs/client-platform/data/unowned_deferrals.json";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits one level below the repository root")
        .to_path_buf()
}

// Every workspace member's `src/` directory, repository-relative, out of `cargo metadata`.
fn member_source_roots(root: &Path) -> Vec<String> {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .expect("running `cargo metadata`");
    assert!(
        output.status.success(),
        "`cargo metadata` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).expect("`cargo metadata` emits UTF-8");
    let metadata = json::parse(&text).expect("`cargo metadata` emits JSON");
    let root = root
        .canonicalize()
        .expect("the repository root exists")
        .to_string_lossy()
        .replace('\\', "/");
    let roots: Vec<String> = metadata
        .get("packages")
        .and_then(Value::array)
        .expect("`cargo metadata` reports a `packages` array")
        .iter()
        .filter_map(|package| package.get("manifest_path").and_then(Value::string))
        .map(|manifest| {
            let manifest = manifest.replace('\\', "/");
            let relative = manifest
                .strip_prefix(&format!("{root}/"))
                .unwrap_or_else(|| panic!("member manifest {manifest} is outside {root}"));
            let directory = relative
                .strip_suffix("Cargo.toml")
                .expect("a manifest path ends in Cargo.toml");
            format!("{directory}src/")
        })
        .collect();
    assert!(
        roots.len() > 20,
        "`cargo metadata` reported {} members",
        roots.len()
    );
    roots
}

// One comment: consecutive comment lines, or a trailing comment and the comment lines under it.
#[derive(Debug)]
struct Comment {
    line: usize,
    lines: Vec<String>,
}

// Where the lexer is at the start of a line.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lexer {
    Code,
    // Inside a string literal; `Some(n)` is a raw string closed by `"` and `n` hashes.
    Text(Option<usize>),
    // Inside a block comment, nested this deep.
    Block(usize),
}

// What one line holds: its comment text, whether nothing but comment is on it, and its code outside literals.
struct Lexed {
    comment: Option<String>,
    whole: bool,
    code: String,
}

// The offset past a block comment's leading `*` gutter or its `/**` or `/*!` doc marker.
fn past_gutter(line: &str, from: usize) -> usize {
    let rest = &line[from..];
    let trimmed = rest.trim_start();
    let at = from + rest.len() - trimmed.len();
    if (trimmed.starts_with('*') || trimmed.starts_with('!')) && !trimmed.starts_with("*/") {
        at + 1
    } else {
        at
    }
}

fn append(comment: &mut Option<String>, text: &str) {
    match comment {
        Some(existing) => {
            existing.push(' ');
            existing.push_str(text);
        }
        None => *comment = Some(text.to_owned()),
    }
}

// Appends `line[from..to]` to `code` when the range is open; every boundary is an ASCII delimiter or a line end.
fn flush(code: &mut String, line: &str, from: &mut Option<usize>, to: usize) {
    if let Some(start) = from.take().filter(|&start| start <= to) {
        code.push_str(&line[start..to]);
    }
}

// Reads one line, carrying string and block-comment state across lines.
fn lex_line(line: &str, state: &mut Lexer) -> Lexed {
    if *state == Lexer::Code {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("//") {
            let body = rest
                .strip_prefix('/')
                .or_else(|| rest.strip_prefix('!'))
                .unwrap_or(rest);
            return Lexed {
                comment: Some(body.to_owned()),
                whole: true,
                code: String::new(),
            };
        }
    }
    let bytes = line.as_bytes();
    let mut comment = None;
    let mut code = String::new();
    let mut block_from = matches!(state, Lexer::Block(_)).then(|| past_gutter(line, 0));
    let mut code_from = (*state == Lexer::Code).then_some(0);
    let mut at = 0;
    while at < bytes.len() {
        match *state {
            Lexer::Block(depth) => {
                if bytes[at..].starts_with(b"*/") {
                    at += 2;
                    if depth == 1 {
                        let start = block_from.take().unwrap_or(at - 2).min(at - 2);
                        append(&mut comment, &line[start..at - 2]);
                        *state = Lexer::Code;
                        code_from = Some(at);
                    } else {
                        *state = Lexer::Block(depth - 1);
                    }
                } else if bytes[at..].starts_with(b"/*") {
                    *state = Lexer::Block(depth + 1);
                    at += 2;
                } else {
                    at += 1;
                }
            }
            Lexer::Text(None) => {
                match bytes[at] {
                    b'\\' => at += 1,
                    b'"' => {
                        *state = Lexer::Code;
                        code_from = Some(at + 1);
                    }
                    _ => {}
                }
                at += 1;
            }
            Lexer::Text(Some(hashes)) => {
                if bytes[at] == b'"'
                    && bytes.len() >= at + 1 + hashes
                    && bytes[at + 1..at + 1 + hashes].iter().all(|&b| b == b'#')
                {
                    *state = Lexer::Code;
                    at += 1 + hashes;
                    code_from = Some(at);
                } else {
                    at += 1;
                }
            }
            Lexer::Code => {
                let rest = &bytes[at..];
                if rest.starts_with(b"//") {
                    flush(&mut code, line, &mut code_from, at);
                    append(&mut comment, &line[at + 2..]);
                    at = bytes.len();
                    continue;
                }
                if rest.starts_with(b"/*") {
                    flush(&mut code, line, &mut code_from, at);
                    *state = Lexer::Block(1);
                    at += 2;
                    block_from = Some(past_gutter(line, at));
                    continue;
                }
                let identifier_before =
                    at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_');
                let byte_prefix = at > 0
                    && bytes[at - 1] == b'b'
                    && (at < 2
                        || !(bytes[at - 2].is_ascii_alphanumeric() || bytes[at - 2] == b'_'));
                if (!identifier_before || byte_prefix)
                    && (rest.starts_with(b"r#") || rest.starts_with(b"r\""))
                {
                    let hashes = rest[1..].iter().take_while(|&&b| b == b'#').count();
                    if rest.get(1 + hashes) == Some(&b'"') {
                        flush(&mut code, line, &mut code_from, at);
                        code.push_str("\"\"");
                        *state = Lexer::Text(Some(hashes));
                        at += 2 + hashes;
                        continue;
                    }
                }
                match bytes[at] {
                    b'"' => {
                        flush(&mut code, line, &mut code_from, at);
                        code.push_str("\"\"");
                        *state = Lexer::Text(None);
                        at += 1;
                    }
                    // A character literal: `'x'` or `'\…'`; a lifetime has no closing quote two bytes on.
                    b'\'' if rest.get(1) == Some(&b'\\') => {
                        let close = rest[2..].iter().position(|&b| b == b'\'');
                        at += close.map_or(1, |close| close + 3);
                    }
                    b'\'' if rest.get(2) == Some(&b'\'') => at += 3,
                    _ => at += 1,
                }
            }
        }
    }
    if let Some(start) = block_from {
        append(&mut comment, &line[start.min(bytes.len())..]);
    }
    flush(&mut code, line, &mut code_from, bytes.len());
    let whole = comment.is_some() && code.trim().is_empty();
    Lexed {
        comment,
        whole,
        code,
    }
}

// The comments in one Rust source.
fn comments_of(source: &str) -> Vec<Comment> {
    let mut state = Lexer::Code;
    let mut comments: Vec<Comment> = Vec::new();
    let mut open = false;
    for (index, line) in source.lines().enumerate() {
        let lexed = lex_line(line, &mut state);
        match lexed.comment {
            Some(body) if lexed.whole && open => comments
                .last_mut()
                .expect("an open comment exists")
                .lines
                .push(body),
            Some(body) => {
                comments.push(Comment {
                    line: index + 1,
                    lines: vec![body],
                });
                open = true;
            }
            None => open = false,
        }
    }
    comments
}

fn normalise(text: &str) -> String {
    text.replace('*', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// One paragraph of a comment: the line it starts on and its normalised text.
#[derive(Debug)]
struct Paragraph {
    line: usize,
    text: String,
}

// A comment's paragraphs, split at blank comment lines and normalised.
fn paragraphs(comment: &Comment) -> Vec<Paragraph> {
    let mut found = Vec::new();
    let mut first = 0;
    let mut current: Vec<&str> = Vec::new();
    for (index, line) in comment.lines.iter().enumerate() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                found.push((first, current.join(" ")));
                current.clear();
            }
            continue;
        }
        if current.is_empty() {
            first = index;
        }
        current.push(line);
    }
    if !current.is_empty() {
        found.push((first, current.join(" ")));
    }
    found
        .into_iter()
        .map(|(index, text)| Paragraph {
            line: comment.line + index,
            text: normalise(&text),
        })
        .filter(|paragraph| !paragraph.text.is_empty())
        .collect()
}

// Byte ranges of a paragraph's sentences, split after `.`, `;`, `?` or `!` and a space.
fn sentences(paragraph: &str) -> Vec<(usize, usize)> {
    let bytes = paragraph.as_bytes();
    let mut ranges = Vec::new();
    let mut start = 0;
    for at in 0..bytes.len() {
        if matches!(bytes[at], b'.' | b';' | b'?' | b'!') && bytes.get(at + 1) == Some(&b' ') {
            ranges.push((start, at + 1));
            start = at + 2;
        }
    }
    if start < bytes.len() {
        ranges.push((start, bytes.len()));
    }
    ranges
}

// Offsets where `needle` occurs in `haystack` as whole words, a plural `s` included.
fn occurrences(haystack: &str, needle: &str) -> Vec<usize> {
    let bytes = haystack.as_bytes();
    haystack
        .match_indices(needle)
        .map(|(at, _)| at)
        .filter(|&at| {
            let before = at.checked_sub(1).map(|i| bytes[i]);
            let end = at + needle.len();
            let plural = bytes.get(end) == Some(&b's');
            let after = bytes.get(if plural { end + 1 } else { end }).copied();
            !before.is_some_and(|b| b.is_ascii_alphanumeric())
                && !after.is_some_and(|b| b.is_ascii_alphanumeric())
        })
        .collect()
}

fn carries_any(text: &str, cues: &[&str]) -> bool {
    cues.iter().any(|cue| !occurrences(text, cue).is_empty())
}

#[derive(Debug)]
struct Finding {
    phrase: &'static str,
    context: String,
}

// Whether an `until` waits on work: a cue after it in its sentence, or `until then` in a paragraph that names the work.
fn waits_on_work(sentence: &str, paragraph: &str, hits: &[usize]) -> bool {
    hits.iter().any(|&at| {
        let after = &sentence[at + "until".len()..];
        carries_any(after, &UNTIL_WORK_CUES)
            || (after.trim_start().starts_with("then") && carries_any(paragraph, &UNTIL_WORK_CUES))
    })
}

// Every deferral phrase one paragraph carries, before exceptions.
fn findings(paragraph: &str) -> Vec<Finding> {
    let lower = paragraph.to_ascii_lowercase();
    let mut found = Vec::new();
    for phrase in &PHRASES {
        let haystack = if phrase.case_sensitive {
            paragraph
        } else {
            lower.as_str()
        };
        if occurrences(haystack, phrase.spelling).is_empty() {
            continue;
        }
        if phrase.rule == Rule::InBlockedParagraph {
            if carries_any(&lower, &BLOCKING_CUES) {
                found.push(Finding {
                    phrase: phrase.spelling,
                    context: paragraph.to_owned(),
                });
            }
            continue;
        }
        for (start, end) in sentences(paragraph) {
            let sentence = &lower[start..end];
            let hits = occurrences(sentence, phrase.spelling);
            let deferral = !hits.is_empty()
                && match phrase.rule {
                    Rule::InTemporarySentence => carries_any(sentence, &TEMPORARY_CUES),
                    Rule::UntilWorkArrives => waits_on_work(sentence, &lower, &hits),
                    Rule::Anywhere | Rule::InBlockedParagraph => true,
                };
            if deferral {
                found.push(Finding {
                    phrase: phrase.spelling,
                    context: paragraph[start..end].to_owned(),
                });
            }
        }
    }
    found
}

// Every `MJXOFF-<n>` a text names.
fn cited_tickets(text: &str) -> BTreeSet<String> {
    text.match_indices("MJXOFF-")
        .filter_map(|(at, prefix)| {
            let digits: String = text[at + prefix.len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            (!digits.is_empty()).then(|| format!("MJXOFF-{digits}"))
        })
        .collect()
}

struct UnownedEntry {
    location: String,
    comment: String,
    feature: String,
}

fn unowned_entries(root: &Path) -> Vec<UnownedEntry> {
    let text = std::fs::read_to_string(root.join(UNOWNED))
        .expect("the unowned-deferral record is committed");
    let document = json::parse(&text).expect("the unowned-deferral record is valid JSON");
    document
        .array()
        .expect("the unowned-deferral record is a JSON array")
        .iter()
        .map(|entry| {
            let field = |key: &str| {
                entry
                    .get(key)
                    .and_then(Value::string)
                    .unwrap_or_else(|| panic!("an unowned deferral has no string `{key}`"))
                    .to_owned()
            };
            UnownedEntry {
                location: field("location"),
                comment: field("comment"),
                feature: field("feature"),
            }
        })
        .collect()
}

// One deferral paragraph found in the tree, after exceptions.
struct Deferral {
    location: String,
    phrases: BTreeSet<&'static str>,
    contexts: Vec<String>,
    cited: BTreeSet<String>,
}

// The deferral paragraphs one source holds after exceptions, marking which exceptions were used.
fn deferrals_in(path: &str, source: &str, used_exceptions: &mut BTreeSet<usize>) -> Vec<Deferral> {
    let mut deferrals = Vec::new();
    for comment in comments_of(source) {
        for paragraph in paragraphs(&comment) {
            let kept: Vec<Finding> = findings(&paragraph.text)
                .into_iter()
                .filter(|finding| {
                    let exception = NOT_DEFERRALS.iter().position(|(file, phrase, excerpt, _)| {
                        *file == path
                            && *phrase == finding.phrase
                            && finding.context.contains(excerpt)
                    });
                    if let Some(index) = exception {
                        used_exceptions.insert(index);
                    }
                    exception.is_none()
                })
                .collect();
            if kept.is_empty() {
                continue;
            }
            deferrals.push(Deferral {
                location: format!("{path}:{}", paragraph.line),
                phrases: kept.iter().map(|finding| finding.phrase).collect(),
                contexts: kept.into_iter().map(|finding| finding.context).collect(),
                cited: cited_tickets(&paragraph.text),
            });
        }
    }
    deferrals
}

// Why a deferral's citations do not own it, or `None` when an open ticket of the roster does.
fn citation_problem(deferral: &Deferral) -> Option<String> {
    let closed: Vec<String> = deferral
        .cited
        .iter()
        .filter_map(|ticket| closed_reason(ticket).map(|reason| format!("{ticket} ({reason})")))
        .collect();
    if !closed.is_empty() {
        return Some(format!("cites closed ticket(s): {}", closed.join("; ")));
    }
    if deferral.cited.iter().any(|ticket| is_open_ticket(ticket)) {
        return None;
    }
    Some(if deferral.cited.is_empty() {
        "cites no ticket in its own paragraph".to_owned()
    } else {
        format!(
            "cites only {:?}, which are not tickets of the roster",
            deferral.cited
        )
    })
}

// Whether a text names an open ticket of the roster and no closed one.
fn owned_by_open_ticket(text: &str) -> bool {
    let cited = cited_tickets(text);
    cited.iter().any(|ticket| is_open_ticket(ticket))
        && cited.iter().all(|ticket| closed_reason(ticket).is_none())
}

// Every `todo!(` or `unimplemented!(` in one source that no open roster ticket owns on its line or in the comment above.
fn placeholders_in(path: &str, source: &str) -> Vec<String> {
    let comments = comments_of(source);
    let mut state = Lexer::Code;
    let mut refused = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let lexed = lex_line(line, &mut state);
        let bytes = lexed.code.as_bytes();
        for placeholder in PLACEHOLDER_MACROS {
            let used = lexed.code.match_indices(placeholder).any(|(at, _)| {
                at == 0 || !(bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_')
            });
            if !used {
                continue;
            }
            let number = index + 1;
            let on_line = lexed.comment.as_deref().is_some_and(owned_by_open_ticket);
            let above = comments
                .iter()
                .find(|comment| comment.line + comment.lines.len() == number)
                .and_then(|comment| paragraphs(comment).pop())
                .is_some_and(|paragraph| owned_by_open_ticket(&paragraph.text));
            if !on_line && !above {
                refused.push(format!(
                    "{path}:{number}: `{placeholder}` cites no open roster ticket on its line or in the comment above"
                ));
            }
        }
    }
    refused
}

// The corpus's deferral comments, the exceptions each used, and how much was read.
// What one sweep of the corpus found and how much it read.
struct Sweep {
    deferrals: Vec<Deferral>,
    used_exceptions: BTreeSet<usize>,
    placeholders: Vec<String>,
    files: usize,
    comments: usize,
}

fn sweep(root: &Path) -> Sweep {
    let tree = WorkingTree::read(root);
    let source_roots = member_source_roots(root);
    let files: Vec<&String> = tree
        .with_extension("rs")
        .filter(|path| {
            source_roots
                .iter()
                .any(|source| path.starts_with(source.as_str()))
        })
        .collect();
    let mut deferrals = Vec::new();
    let mut used_exceptions = BTreeSet::new();
    let mut placeholders = Vec::new();
    let mut comment_count = 0;
    for path in &files {
        let source = std::fs::read_to_string(root.join(path))
            .unwrap_or_else(|error| panic!("reading {path}: {error}"));
        comment_count += comments_of(&source).len();
        deferrals.extend(deferrals_in(path, &source, &mut used_exceptions));
        placeholders.extend(placeholders_in(path, &source));
    }
    println!("{}", tree.census());
    Sweep {
        deferrals,
        used_exceptions,
        placeholders,
        files: files.len(),
        comments: comment_count,
    }
}

#[test]
fn every_deferral_cites_its_owning_ticket() {
    let root = repository_root();
    let Sweep {
        deferrals,
        used_exceptions,
        placeholders,
        files,
        comments,
    } = sweep(&root);
    assert!(
        files >= MINIMUM_FILES,
        "only {files} source file(s) were swept"
    );
    assert!(
        comments >= MINIMUM_COMMENTS,
        "only {comments} comment(s) were read"
    );
    let unowned = unowned_entries(&root);
    let mut matched_unowned = BTreeSet::new();
    let mut per_ticket: BTreeMap<String, usize> = BTreeMap::new();
    let mut per_phrase: BTreeMap<&str, usize> = BTreeMap::new();
    let mut cited = 0;
    let mut failures = placeholders;
    for deferral in &deferrals {
        for phrase in &deferral.phrases {
            *per_phrase.entry(phrase).or_default() += 1;
        }
        let Some(problem) = citation_problem(deferral) else {
            cited += 1;
            for ticket in deferral
                .cited
                .iter()
                .filter(|ticket| is_open_ticket(ticket))
            {
                *per_ticket.entry(ticket.clone()).or_default() += 1;
            }
            continue;
        };
        let cites_closed = deferral
            .cited
            .iter()
            .any(|ticket| closed_reason(ticket).is_some());
        let recorded = unowned.iter().position(|entry| {
            entry.location == deferral.location && deferral.contexts.contains(&entry.comment)
        });
        if let (false, Some(index)) = (cites_closed, recorded) {
            matched_unowned.insert(index);
            continue;
        }
        failures.push(format!(
            "{} {:?} {problem}\n    {}",
            deferral.location,
            deferral.phrases,
            deferral.contexts.join("\n    ")
        ));
    }
    for (index, entry) in unowned.iter().enumerate() {
        if !matched_unowned.contains(&index) {
            failures.push(format!(
                "{UNOWNED}: {} no longer holds an uncited deferral reading {:?}",
                entry.location, entry.comment
            ));
        }
    }
    for (index, (file, phrase, excerpt, _)) in NOT_DEFERRALS.iter().enumerate() {
        if !used_exceptions.contains(&index) {
            failures.push(format!(
                "exception {file} {phrase:?} {excerpt:?} matched nothing and is stale"
            ));
        }
    }
    println!(
        "{files} files, {comments} comments, {} deferrals: {} cited, {} unowned; by phrase {per_phrase:?}; by ticket {per_ticket:?}",
        deferrals.len(),
        cited,
        matched_unowned.len()
    );
    assert!(
        failures.is_empty(),
        "{} deferral problem(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn no_deferral_is_left_unowned() {
    let entries = unowned_entries(&repository_root());
    assert!(
        entries.is_empty(),
        "{} deferral(s) have no owning ticket yet, in {UNOWNED}:\n{}",
        entries.len(),
        entries
            .iter()
            .map(|entry| format!("{}: {}", entry.location, entry.feature))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn every_phrase_and_exception_states_its_reason() {
    let mut failures = Vec::new();
    for phrase in &PHRASES {
        if phrase.reason.trim().is_empty() {
            failures.push(format!("phrase {:?} states no reason", phrase.spelling));
        }
        if !phrase.case_sensitive && phrase.spelling != phrase.spelling.to_ascii_lowercase() {
            failures.push(format!(
                "phrase {:?} is case-insensitive but not lower case",
                phrase.spelling
            ));
        }
    }
    let reasons = NOT_DEFERRALS
        .iter()
        .map(|(file, _, _, reason)| (*file, *reason));
    for (subject, reason) in reasons {
        if reason.trim().is_empty() {
            failures.push(format!("{subject:?} states no reason"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// Runs the reader over one synthetic source and returns each paragraph's phrases.
fn phrases_in(source: &str) -> Vec<Vec<&'static str>> {
    comments_of(source)
        .iter()
        .flat_map(paragraphs)
        .map(|paragraph| {
            findings(&paragraph.text)
                .iter()
                .map(|finding| finding.phrase)
                .collect()
        })
        .collect()
}

#[test]
fn the_reader_tells_a_deferral_from_ordinary_prose() {
    let deferrals = [
        "// Run colour is not built yet, so this is a stated gap.",
        "// This engine does not yet read `c:marker`.",
        "/// A shape drawn as a stand-in until the table exists.",
        "// These shapes are stand-ins until the table exists.",
        "// GUESS: flush left. Real justification needs a second pass.",
        "// **GUESS:** kashida needs a shaper feature nothing drives yet.",
        "// The pixels are deliberately not drawn for now.",
        "let x = 1; // the rest is owed to later work",
    ];
    for source in deferrals {
        assert!(
            phrases_in(source).iter().any(|phrases| !phrases.is_empty()),
            "missed a deferral: {source}"
        );
    }
    let prose = [
        "/// Deliberately not a JSON parser: the body is two fields.",
        "/// Whether stand-in geometry is drawn in the warning colour.",
        "// GUESS: Office's default marker is seven points across.",
        "// This guess is cannot yet-ish, and GUESSED is not a marker.",
        "let url = \"https://example.com/a stated gap\";",
        "let quote = '\"'; let s = \"// not yet\";",
        "let raw = r#\"a \" // stated gap\"#;",
    ];
    for source in prose {
        assert!(
            phrases_in(source).iter().all(Vec::is_empty),
            "read ordinary prose as a deferral: {source} -> {:?}",
            phrases_in(source)
        );
    }
}

#[test]
fn a_comment_is_its_consecutive_lines_split_into_paragraphs() {
    let source = "fn a() {}\n/// A stated gap in the\n///\n/// run colour, owned by MJXOFF-311.\nfn b() {}\n// A separate stated gap.\n";
    let comments = comments_of(source);
    assert_eq!(comments.len(), 2, "{comments:?}");
    assert_eq!(comments[0].line, 2);
    let split = paragraphs(&comments[0]);
    assert_eq!(
        split
            .iter()
            .map(|paragraph| paragraph.line)
            .collect::<Vec<_>>(),
        [2, 4]
    );
    assert!(cited_tickets(&split[0].text).is_empty());
    assert_eq!(
        cited_tickets(&split[1].text),
        BTreeSet::from(["MJXOFF-311".to_owned()])
    );
    let multiline = "let s = \"first\n// inside a string\nlast\"; // outside\n";
    let read = comments_of(multiline);
    assert_eq!(read.len(), 1, "{read:?}");
    assert_eq!(read[0].line, 3);
    assert!(is_open_ticket("MJXOFF-353") && is_open_ticket("MJXOFF-311"));
    assert!(!is_open_ticket("MJXOFF-177") && !is_open_ticket("MJXOFF-297"));
}

// The deferrals a synthetic source holds at a path no exception names.
fn deferrals_of(source: &str) -> Vec<Deferral> {
    deferrals_in("crates/synthetic/src/lib.rs", source, &mut BTreeSet::new())
}

#[test]
fn the_widened_phrases_are_read() {
    let deferrals = [
        "// A picture bullet is not drawn.",
        "/// The staggered arrangement is not implemented.",
        "/// Resolving that is a separate, future task.",
        "//! The transports are a later unit too.",
        "//! `mjx-pptx` keeps its own reader for now.",
        "// The pixels wait for artwork somebody is entitled to ship.",
        "// `w:rtlGutter` is not honoured here.",
        "// That is a ticket of its own; until it exists, a value is formatted by its axis step.",
        "// GUESS: the staggered arrangement waits until `mjx-text` reads the MATH table.",
        "// GUESS: the staggered arrangement is not implemented.",
        "// GUESS: aligned, because `mjx-text` reads no such table.",
        "/// GUESS: 80 %. A font states its own; `mjx-text` reads none, so this is the fallback.",
        "/// GUESS: a comma, because a locale-aware separator needs the number-format language.",
        "/// GUESS: `heavy` is bolder, which this cannot express, so it uses the same glyph.",
        "/// GUESS: the base format stays; it is the one member this build does not honour.",
    ];
    for source in deferrals {
        assert!(
            phrases_in(source).iter().any(|phrases| !phrases.is_empty()),
            "missed a deferral: {source}"
        );
    }
    for prose in [
        "/// Builds a new, empty `w:u`, every attribute absent until a setter states one.",
        "// Repeat until it fits.",
        "// GUESS: Office's default marker is seven points across.",
        "// A window is built from a count, which is an estimate until a page reports no continuation.",
        "// A variant stops both hosts compiling until someone decides; the rule exists to forbid it.",
    ] {
        assert!(
            phrases_in(prose).iter().all(Vec::is_empty),
            "read ordinary prose as a deferral: {prose} -> {:?}",
            phrases_in(prose)
        );
    }
}

#[test]
fn a_citation_covers_only_its_own_paragraph() {
    let source = "//! A session crate, filed as MJXOFF-311.\n//!\n//! Selection is a stated gap.\n//!\n//! Undo is a stated gap, owned by MJXOFF-353.\n";
    let deferrals = deferrals_of(source);
    let problems: Vec<(&str, Option<String>)> = deferrals
        .iter()
        .map(|deferral| (deferral.location.as_str(), citation_problem(deferral)))
        .collect();
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert_eq!(problems[0].0, "crates/synthetic/src/lib.rs:3");
    assert!(problems[0].1.is_some(), "{problems:?}");
    assert_eq!(problems[1].0, "crates/synthetic/src/lib.rs:5");
    assert!(problems[1].1.is_none(), "{problems:?}");
}

#[test]
fn a_closed_ticket_owns_no_deferral() {
    for closed in ["MJXOFF-296", "MJXOFF-297", "MJXOFF-298"] {
        let source = format!("// A stated gap, owned by {closed}.");
        let deferrals = deferrals_of(&source);
        assert_eq!(deferrals.len(), 1, "{source}");
        let problem = citation_problem(&deferrals[0]);
        assert!(
            problem
                .as_deref()
                .is_some_and(|problem| problem.contains("closed")),
            "{source} -> {problem:?}"
        );
    }
    let both = deferrals_of("// A stated gap, owned by MJXOFF-311 and once by MJXOFF-297.");
    assert!(citation_problem(&both[0]).is_some());
}

#[test]
fn a_state_word_exempts_only_the_occurrence_it_is_listed_for() {
    for source in [
        "/// One header part's paragraphs, read but not yet resolved.",
        "// A text body autofit has not yet had to scale.",
        "// Bytes not yet parsed.",
        "// A line not yet given its position.",
    ] {
        assert!(
            !deferrals_of(source).is_empty(),
            "a state word exempted an unlisted occurrence: {source}"
        );
    }
}

#[test]
fn block_comments_and_placeholder_macros_are_read() {
    for source in [
        "fn a() {} /* Run colour is a stated gap. */",
        "/**\n * Run colour is\n * not built yet.\n */\nfn a() {}",
        "/*! A stated\n gap. /* nested */ still */",
        "/* One paragraph.\n\n A second holds a stated gap. */",
    ] {
        assert!(
            !deferrals_of(source).is_empty(),
            "missed a block-comment deferral: {source:?}"
        );
    }
    assert!(deferrals_of("let s = \"/* a stated gap */\";").is_empty());
    let path = "crates/synthetic/src/lib.rs";
    for source in [
        "fn a() { todo!() }",
        "fn a() {\n    unimplemented!(\"later\")\n}",
        "// Owned by MJXOFF-297.\nfn a() { todo!() }",
        "// Owned by MJXOFF-311.\n\nfn a() { todo!() }",
    ] {
        assert_eq!(placeholders_in(path, source).len(), 1, "{source:?}");
    }
    for source in [
        "fn a() { todo!() } // Owned by MJXOFF-311.",
        "// Owned by MJXOFF-311.\nfn a() { todo!() }",
        "let s = \"todo!()\"; // not a macro",
        "fn my_todo!() {}",
    ] {
        assert!(placeholders_in(path, source).is_empty(), "{source:?}");
    }
}
