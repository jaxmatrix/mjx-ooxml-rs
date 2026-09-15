//! The deferral gate (MJXOFF-298): a comment in a workspace member's `src/` that defers a feature cites the ticket that owns it, or is recorded in `docs/client-platform/data/unowned_deferrals.json`.

#[path = "../src/json.rs"]
mod json;
#[path = "../src/ticket_roster.rs"]
mod ticket_roster;

use json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use ticket_roster::is_known_ticket;
use xtask::repository_files::WorkingTree;

// How a phrase's occurrence becomes a deferral.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Rule {
    // Every occurrence is one.
    Anywhere,
    // Unless the next word, past `been` or `be`, is in STATE_WORDS.
    UnlessStateFollows,
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

// The deferral phrases, each with why it marks a deferral and how narrowly it is read.
const PHRASES: [Phrase; 8] = [
    Phrase {
        spelling: "stated gap",
        case_sensitive: false,
        rule: Rule::Anywhere,
        reason: "admits behaviour the code knows it lacks",
    },
    Phrase {
        spelling: "stated limitation",
        case_sensitive: false,
        rule: Rule::Anywhere,
        reason: "loop 1's second spelling of a stated gap, in the same sentence shape",
    },
    Phrase {
        spelling: "not built yet",
        case_sensitive: false,
        rule: Rule::Anywhere,
        reason: "says the missing piece is still to be written",
    },
    Phrase {
        spelling: "is owed to",
        case_sensitive: false,
        rule: Rule::Anywhere,
        reason: "hands the missing piece to later work",
    },
    Phrase {
        spelling: "not yet",
        case_sensitive: false,
        rule: Rule::UnlessStateFollows,
        reason: "says a capability is still to come, unless it describes a value in flight",
    },
    Phrase {
        spelling: "deliberately not",
        case_sensitive: false,
        rule: Rule::InTemporarySentence,
        reason: "a design decision, and a deferral only when its sentence says it is temporary",
    },
    Phrase {
        spelling: "stand-in",
        case_sensitive: false,
        rule: Rule::InTemporarySentence,
        reason: "the placeholder-geometry vocabulary, and a deferral only when its sentence says it is temporary",
    },
    Phrase {
        spelling: "GUESS",
        case_sensitive: true,
        rule: Rule::InBlockedParagraph,
        reason: "an unverified reading of Office, and a deferral only when its paragraph says the real answer is blocked",
    },
];

// Words after `not yet` that describe a value partway through a computation, with why each is runtime state.
const STATE_WORDS: [(&str, &str); 13] = [
    (
        "calculated",
        "a guide's evaluation order, quoted from ECMA-376",
    ),
    ("committed", "a working-tree file awaiting its commit"),
    ("flushed", "journal records not yet written to the sink"),
    ("freed", "allocator bytes still live"),
    ("given", "a composed line awaiting its position"),
    ("had", "a text body autofit has not needed to scale"),
    ("handed", "journal records not yet written to the sink"),
    ("laid", "content awaiting layout within one pass"),
    ("parsed", "part bytes awaiting tokenisation"),
    ("placed", "a line awaiting its page"),
    (
        "resolved",
        "a part read and awaiting resolution within one pass",
    ),
    ("turned", "a node awaiting its fragment"),
    ("warm", "a cache's state at run time"),
];

// Sentence words that make a decision or a placeholder temporary.
const TEMPORARY_CUES: [&str; 4] = ["yet", "until", "for now", "for the moment"];

// Paragraph phrases that say a GUESS stands in for an answer something missing blocks.
const BLOCKING_CUES: [&str; 11] = [
    "yet",
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
const NOT_DEFERRALS: [(&str, &str, &str, &str); 10] = [
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
    Block,
}

// The comment text a line carries, and whether the whole line is a comment.
fn line_comment(line: &str, state: &mut Lexer) -> Option<(String, bool)> {
    if *state == Lexer::Code {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("//") {
            let body = rest
                .strip_prefix('/')
                .or_else(|| rest.strip_prefix('!'))
                .unwrap_or(rest);
            return Some((body.to_owned(), true));
        }
    }
    let bytes = line.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        match *state {
            Lexer::Block => {
                if bytes[at..].starts_with(b"*/") {
                    *state = Lexer::Code;
                    at += 2;
                } else {
                    at += 1;
                }
            }
            Lexer::Text(None) => {
                match bytes[at] {
                    b'\\' => at += 1,
                    b'"' => *state = Lexer::Code,
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
                } else {
                    at += 1;
                }
            }
            Lexer::Code => {
                let rest = &bytes[at..];
                if rest.starts_with(b"//") {
                    return Some((line[at + 2..].to_owned(), false));
                }
                if rest.starts_with(b"/*") {
                    *state = Lexer::Block;
                    at += 2;
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
                        *state = Lexer::Text(Some(hashes));
                        at += 2 + hashes;
                        continue;
                    }
                }
                match bytes[at] {
                    b'"' => {
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
    None
}

// The comments in one Rust source.
fn comments_of(source: &str) -> Vec<Comment> {
    let mut state = Lexer::Code;
    let mut comments: Vec<Comment> = Vec::new();
    let mut open = false;
    for (index, line) in source.lines().enumerate() {
        match line_comment(line, &mut state) {
            Some((body, true)) if open => comments
                .last_mut()
                .expect("an open comment exists")
                .lines
                .push(body),
            Some((body, _)) => {
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

// A comment's paragraphs, split at blank comment lines and normalised.
fn paragraphs(comment: &Comment) -> Vec<String> {
    comment
        .lines
        .split(|line| line.trim().is_empty())
        .map(|lines| normalise(&lines.join(" ")))
        .filter(|paragraph| !paragraph.is_empty())
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

// The word after byte `from`, past `been` or `be`.
fn next_word(lower: &str, from: usize) -> &str {
    let mut words = lower[from..]
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty());
    match words.next() {
        Some("been" | "be") => words.next().unwrap_or(""),
        other => other.unwrap_or(""),
    }
}

#[derive(Debug)]
struct Finding {
    phrase: &'static str,
    context: String,
}

// The sentence of `paragraph` holding byte `at`.
fn sentence_at(paragraph: &str, at: usize) -> String {
    sentences(paragraph)
        .into_iter()
        .find(|&(start, end)| start <= at && at < end)
        .map_or_else(
            || paragraph.to_owned(),
            |(start, end)| paragraph[start..end].to_owned(),
        )
}

// Every deferral phrase a comment's paragraphs carry, before exceptions.
fn findings(comment: &Comment) -> Vec<Finding> {
    let mut found = Vec::new();
    for paragraph in paragraphs(comment) {
        let lower = paragraph.to_ascii_lowercase();
        for phrase in &PHRASES {
            let haystack = if phrase.case_sensitive {
                paragraph.as_str()
            } else {
                lower.as_str()
            };
            let hits = occurrences(haystack, phrase.spelling);
            match phrase.rule {
                Rule::Anywhere => found.extend(hits.iter().map(|&at| Finding {
                    phrase: phrase.spelling,
                    context: sentence_at(&paragraph, at),
                })),
                Rule::UnlessStateFollows => {
                    found.extend(
                        hits.iter()
                            .filter(|&&at| {
                                let word = next_word(&lower, at + phrase.spelling.len());
                                !STATE_WORDS.iter().any(|(state, _)| *state == word)
                            })
                            .map(|&at| Finding {
                                phrase: phrase.spelling,
                                context: sentence_at(&paragraph, at),
                            }),
                    );
                }
                Rule::InTemporarySentence => {
                    for (start, end) in sentences(&paragraph) {
                        let sentence = &lower[start..end];
                        if !occurrences(sentence, phrase.spelling).is_empty()
                            && TEMPORARY_CUES
                                .iter()
                                .any(|cue| !occurrences(sentence, cue).is_empty())
                        {
                            found.push(Finding {
                                phrase: phrase.spelling,
                                context: paragraph[start..end].to_owned(),
                            });
                        }
                    }
                }
                Rule::InBlockedParagraph => {
                    if !hits.is_empty()
                        && BLOCKING_CUES
                            .iter()
                            .any(|cue| !occurrences(&lower, cue).is_empty())
                    {
                        found.push(Finding {
                            phrase: phrase.spelling,
                            context: paragraph.clone(),
                        });
                    }
                }
            }
        }
    }
    found
}

// Every `MJXOFF-<n>` a comment names.
fn cited_tickets(comment: &Comment) -> BTreeSet<String> {
    let text = comment.lines.join(" ");
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

// One deferral comment found in the tree, after exceptions.
struct Deferral {
    location: String,
    phrases: BTreeSet<&'static str>,
    contexts: Vec<String>,
    cited: BTreeSet<String>,
}

// The corpus's deferral comments, the exceptions each used, and how much was read.
fn sweep(root: &Path) -> (Vec<Deferral>, BTreeSet<usize>, usize, usize) {
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
    let mut comment_count = 0;
    for path in &files {
        let source = std::fs::read_to_string(root.join(path))
            .unwrap_or_else(|error| panic!("reading {path}: {error}"));
        for comment in comments_of(&source) {
            comment_count += 1;
            let kept: Vec<Finding> = findings(&comment)
                .into_iter()
                .filter(|finding| {
                    let exception = NOT_DEFERRALS.iter().position(|(file, phrase, excerpt, _)| {
                        file == path
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
                location: format!("{path}:{}", comment.line),
                phrases: kept.iter().map(|finding| finding.phrase).collect(),
                contexts: kept.into_iter().map(|finding| finding.context).collect(),
                cited: cited_tickets(&comment),
            });
        }
    }
    println!("{}", tree.census());
    (deferrals, used_exceptions, files.len(), comment_count)
}

#[test]
fn every_deferral_cites_its_owning_ticket() {
    let root = repository_root();
    let (deferrals, used_exceptions, files, comments) = sweep(&root);
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
    let mut failures = Vec::new();
    for deferral in &deferrals {
        for phrase in &deferral.phrases {
            *per_phrase.entry(phrase).or_default() += 1;
        }
        let known: Vec<&String> = deferral
            .cited
            .iter()
            .filter(|ticket| is_known_ticket(ticket))
            .collect();
        if !known.is_empty() {
            for ticket in known {
                *per_ticket.entry(ticket.clone()).or_default() += 1;
            }
            continue;
        }
        let recorded = unowned.iter().position(|entry| {
            entry.location == deferral.location && deferral.contexts.contains(&entry.comment)
        });
        if let Some(index) = recorded {
            matched_unowned.insert(index);
            continue;
        }
        let outside = if deferral.cited.is_empty() {
            String::new()
        } else {
            format!(
                " (cites only {:?}, which are not tickets of the roster)",
                deferral.cited
            )
        };
        failures.push(format!(
            "{} {:?}{outside}\n    {}",
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
        per_ticket.values().sum::<usize>(),
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
    let reasons = STATE_WORDS
        .iter()
        .map(|(word, reason)| (*word, *reason))
        .chain(
            NOT_DEFERRALS
                .iter()
                .map(|(file, _, _, reason)| (*file, *reason)),
        );
    for (subject, reason) in reasons {
        if reason.trim().is_empty() {
            failures.push(format!("{subject:?} states no reason"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// Runs the reader over one synthetic source and returns each comment's phrases.
fn phrases_in(source: &str) -> Vec<Vec<&'static str>> {
    comments_of(source)
        .iter()
        .map(|comment| {
            findings(comment)
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
        "// Answers early if the cache is not yet warm... not yet laid out, not yet been handed over.",
        "/// One header part's paragraphs, read but not yet resolved.",
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
fn a_comment_is_its_consecutive_lines_and_a_citation_anywhere_in_them_counts() {
    let source = "fn a() {}\n/// A stated gap in the\n///\n/// run colour, owned by MJXOFF-311.\nfn b() {}\n// A separate stated gap.\n";
    let comments = comments_of(source);
    assert_eq!(comments.len(), 2, "{comments:?}");
    assert_eq!(comments[0].line, 2);
    assert_eq!(
        cited_tickets(&comments[0]),
        BTreeSet::from(["MJXOFF-311".to_owned()])
    );
    assert!(cited_tickets(&comments[1]).is_empty());
    let multiline = "let s = \"first\n// inside a string\nlast\"; // outside\n";
    let read = comments_of(multiline);
    assert_eq!(read.len(), 1, "{read:?}");
    assert_eq!(read[0].line, 3);
    assert!(is_known_ticket("MJXOFF-348") && is_known_ticket("MJXOFF-311"));
    assert!(!is_known_ticket("MJXOFF-177"));
}
