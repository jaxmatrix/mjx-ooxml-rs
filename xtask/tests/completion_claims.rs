//! The completion-claim gate (MJXOFF-298): a commit message since `main` or a document under `docs/` that says a format or the renderer is complete needs a passed RC46 acceptance record for it.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use xtask::repository_files::WorkingTree;

// The branch whose commits since its merge-base with the base are read, overridable for a scratch branch.
const BRANCH_VARIABLE: &str = "MJX_COMPLETION_CLAIMS_BRANCH";
const DEFAULT_BRANCH: &str = "epic/renderer-completion";
const BASE_VARIABLE: &str = "MJX_COMPLETION_CLAIMS_BASE";
const DEFAULT_BASE: &str = "main";

// An acceptance record lives at this directory, one Markdown file per format named after it.
const ACCEPTANCE_DIRECTORY: &str = "docs/client-platform/acceptance";

// A record opens with this HTML comment, holds one `key: value` line per RECORD_KEYS entry, and closes with `-->`.
const RECORD_OPENING: &str = "<!-- mjx-acceptance";
const RECORD_CLOSING: &str = "-->";
const RECORD_KEYS: [&str; 7] = [
    "format",
    "gate",
    "date",
    "approver",
    "fixture",
    "verdict",
    "windows-run",
];

// The only gate a record may state, and the verdicts it may carry; only `passed` backs a claim.
const ACCEPTANCE_GATE: &str = "RC46";
const VERDICTS: [&str; 2] = ["passed", "failed"];
const FORMATS: [&str; 3] = ["pptx", "docx", "xlsx"];

// The words a claim's subject is spelled with, formats and products first, and the formats a claim naming each needs accepted.
const SUBJECTS: [(&str, &[&str]); 8] = [
    ("pptx", &["pptx"]),
    ("PowerPoint", &["pptx"]),
    ("docx", &["docx"]),
    ("Word", &["docx"]),
    ("xlsx", &["xlsx"]),
    ("Excel", &["xlsx"]),
    ("renderer", &["pptx", "docx", "xlsx"]),
    ("Phase R", &["pptx", "docx", "xlsx"]),
];

// How many leading SUBJECTS entries are format or product names, which alone may qualify a subject noun.
const PRODUCT_SUBJECTS: usize = 6;

// The predicates a completion claim is written with.
const PREDICATES: [&str; 3] = [" is complete", " is now complete", " are complete"];

// Nouns that may follow a format name in a claim's subject, as in "the pptx renderer is complete".
const SUBJECT_NOUNS: [&str; 3] = ["renderer", "rendering", "support"];

// Claims that are not a present completion claim: a document path or commit hash prefix, an excerpt of the claim, and why.
const EXCEPTIONS: [(&str, &str, &str); 0] = [];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits one level below the repository root")
        .to_path_buf()
}

#[derive(Debug, PartialEq, Eq)]
enum HistoryError {
    // The history is shallow, so the merge-base may not be present.
    Shallow,
    // A ref resolves neither locally nor under `origin/`.
    MissingRef(String),
    // `git` failed for another reason.
    Git(String),
}

impl fmt::Display for HistoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shallow => write!(
                formatter,
                "the clone is shallow, so the merge-base with the base branch may be missing; run `git fetch --unshallow`"
            ),
            Self::MissingRef(name) => write!(
                formatter,
                "the ref `{name}` resolves neither locally nor as `origin/{name}`; fetch it, or set {BRANCH_VARIABLE} / {BASE_VARIABLE}"
            ),
            Self::Git(message) => write!(formatter, "git failed: {message}"),
        }
    }
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, HistoryError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|error| HistoryError::Git(format!("running git {arguments:?}: {error}")))?;
    if !output.status.success() {
        return Err(HistoryError::Git(format!(
            "git {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| HistoryError::Git(format!("git {arguments:?} emitted non-UTF-8: {error}")))
}

// The commit a ref names, trying `origin/<ref>` when the local ref is absent.
fn resolve(root: &Path, name: &str) -> Result<String, HistoryError> {
    for candidate in [name.to_owned(), format!("origin/{name}")] {
        let spec = format!("{candidate}^{{commit}}");
        if let Ok(hash) = git(root, &["rev-parse", "--verify", "--quiet", &spec]) {
            return Ok(hash.trim().to_owned());
        }
    }
    Err(HistoryError::MissingRef(name.to_owned()))
}

// Each commit reachable from `branch` after its merge-base with `base`, as hash and message.
fn commits_since_base(
    root: &Path,
    branch: &str,
    base: &str,
) -> Result<Vec<(String, String)>, HistoryError> {
    if git(root, &["rev-parse", "--is-shallow-repository"])?.trim() == "true" {
        return Err(HistoryError::Shallow);
    }
    let branch = resolve(root, branch)?;
    let base = resolve(root, base)?;
    let merge_base = git(root, &["merge-base", &base, &branch])?;
    let range = format!("{}..{branch}", merge_base.trim());
    let log = git(root, &["log", "--format=%H%x00%B%x1e", &range])?;
    Ok(log
        .split('\u{1e}')
        .filter_map(|entry| entry.trim_start().split_once('\0'))
        .map(|(hash, message)| (hash.to_owned(), message.to_owned()))
        .collect())
}

fn trim_word(word: &str) -> &str {
    let word = word.trim_matches(|c: char| "*`_\"'()[]:;.,!?—".contains(c));
    word.strip_suffix("'s").unwrap_or(word)
}

// Whether `word` spells `subject`: format and product names exactly or in capitals, other subjects in any case.
fn spells(word: &str, subject: &str) -> bool {
    let word = word.trim_start_matches('.');
    if subject.chars().next().is_some_and(char::is_uppercase) {
        word == subject || word == subject.to_uppercase()
    } else {
        word.eq_ignore_ascii_case(subject)
    }
}

// The formats each completion claim in `text` needs accepted, with the sentence it was read from.
fn claims(text: &str) -> Vec<(BTreeSet<&'static str>, String)> {
    let lower = text.to_ascii_lowercase();
    let mut found = Vec::new();
    for predicate in PREDICATES {
        for (at, _) in lower.match_indices(predicate) {
            let after = lower.as_bytes().get(at + predicate.len());
            if after.is_some_and(u8::is_ascii_alphanumeric) {
                continue;
            }
            let line_start = text[..at].rfind('\n').map_or(0, |index| index + 1);
            let words: Vec<&str> = text[line_start..at]
                .split_whitespace()
                .map(trim_word)
                .filter(|word| !word.is_empty())
                .collect();
            let mut formats = BTreeSet::new();
            let mut index = words.len();
            // A subject noun may close the subject, as long as a format or product name precedes it.
            let mut noun = false;
            while index > 0 {
                let word = words[index - 1];
                let two = (index >= 2).then(|| format!("{} {word}", words[index - 2]));
                let qualified = index >= 2
                    && SUBJECTS[..PRODUCT_SUBJECTS]
                        .iter()
                        .any(|(subject, _)| spells(words[index - 2], subject));
                if !noun
                    && formats.is_empty()
                    && qualified
                    && SUBJECT_NOUNS.contains(&word.to_ascii_lowercase().as_str())
                {
                    noun = true;
                    index -= 1;
                } else if let Some((_, needs)) = SUBJECTS.iter().find(|(subject, _)| {
                    spells(word, subject) || two.as_deref().is_some_and(|two| spells(two, subject))
                }) {
                    formats.extend(needs.iter().copied());
                    index -= if two
                        .as_deref()
                        .is_some_and(|two| two.eq_ignore_ascii_case("phase r"))
                    {
                        2
                    } else {
                        1
                    };
                } else if !formats.is_empty() && ["and", "&", "or"].contains(&word) {
                    index -= 1;
                } else {
                    break;
                }
            }
            if !formats.is_empty() {
                let line_end = text[at..].find('\n').map_or(text.len(), |index| at + index);
                found.push((formats, text[line_start..line_end].trim().to_owned()));
            }
        }
    }
    found
}

#[derive(Debug)]
struct Record {
    format: String,
    verdict: String,
}

// Parses an acceptance record for `format`, refusing any header the format definition above does not allow.
fn parse_record(text: &str, format: &str) -> Result<Record, String> {
    let body = text
        .trim_start()
        .strip_prefix(RECORD_OPENING)
        .ok_or_else(|| format!("does not open with `{RECORD_OPENING}`"))?;
    let (header, _) = body
        .split_once(RECORD_CLOSING)
        .ok_or_else(|| format!("the header is not closed with `{RECORD_CLOSING}`"))?;
    let mut fields: Vec<(String, String)> = Vec::new();
    for line in header
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| format!("header line {line:?} is not `key: value`"))?;
        let (key, value) = (key.trim(), value.trim());
        if !RECORD_KEYS.contains(&key) {
            return Err(format!("unknown header key `{key}`"));
        }
        if fields.iter().any(|(seen, _)| seen == key) {
            return Err(format!("header key `{key}` is stated twice"));
        }
        if value.is_empty() {
            return Err(format!("header key `{key}` is empty"));
        }
        fields.push((key.to_owned(), value.to_owned()));
    }
    let field = |key: &str| {
        fields
            .iter()
            .find(|(seen, _)| seen == key)
            .map(|(_, value)| value.as_str())
            .ok_or_else(|| format!("header key `{key}` is missing"))
    };
    for key in RECORD_KEYS {
        field(key)?;
    }
    if field("format")? != format {
        return Err(format!(
            "states format `{}` in the record for `{format}`",
            field("format")?
        ));
    }
    if field("gate")? != ACCEPTANCE_GATE {
        return Err(format!(
            "states gate `{}`, not `{ACCEPTANCE_GATE}`",
            field("gate")?
        ));
    }
    let verdict = field("verdict")?;
    if !VERDICTS.contains(&verdict) {
        return Err(format!("verdict `{verdict}` is not one of {VERDICTS:?}"));
    }
    let date = field("date")?;
    let parts: Vec<&str> = date.split('-').collect();
    let valid_date = parts.len() == 3
        && [4, 2, 2].iter().zip(&parts).all(|(length, part)| {
            part.len() == *length && part.bytes().all(|byte| byte.is_ascii_digit())
        })
        && (1..=12).contains(&parts[1].parse::<u32>().unwrap_or(0))
        && (1..=31).contains(&parts[2].parse::<u32>().unwrap_or(0));
    if !valid_date {
        return Err(format!("date `{date}` is not YYYY-MM-DD"));
    }
    Ok(Record {
        format: format.to_owned(),
        verdict: verdict.to_owned(),
    })
}

// The formats whose acceptance record exists, parses, and records a passed gate.
fn accepted_formats(root: &Path) -> BTreeSet<String> {
    FORMATS
        .iter()
        .filter_map(|format| {
            let text = std::fs::read_to_string(
                root.join(ACCEPTANCE_DIRECTORY).join(format!("{format}.md")),
            )
            .ok()?;
            parse_record(&text, format).ok()
        })
        .filter(|record| record.verdict == "passed")
        .map(|record| record.format)
        .collect()
}

// The refusal for each unaccepted claim in `text` from `source`, marking which exceptions were used.
fn refusals(
    source: &str,
    text: &str,
    accepted: &BTreeSet<String>,
    used: &mut BTreeSet<usize>,
) -> Vec<String> {
    let mut refused = Vec::new();
    for (formats, sentence) in claims(text) {
        let missing: Vec<&str> = formats
            .iter()
            .copied()
            .filter(|format| !accepted.contains(*format))
            .collect();
        if missing.is_empty() {
            continue;
        }
        let exception = EXCEPTIONS.iter().position(|(prefix, excerpt, _)| {
            source.starts_with(prefix) && sentence.contains(excerpt)
        });
        if let Some(index) = exception {
            used.insert(index);
            continue;
        }
        refused.push(format!(
            "{source}: {sentence:?} claims completion with no passed {ACCEPTANCE_GATE} record for {missing:?}"
        ));
    }
    refused
}

fn environment_or(variable: &str, default: &str) -> String {
    std::env::var(variable).unwrap_or_else(|_| default.to_owned())
}

#[test]
fn no_commit_since_the_base_claims_an_unaccepted_completion() {
    let root = repository_root();
    let branch = environment_or(BRANCH_VARIABLE, DEFAULT_BRANCH);
    let base = environment_or(BASE_VARIABLE, DEFAULT_BASE);
    let commits = match commits_since_base(&root, &branch, &base) {
        Ok(commits) => commits,
        Err(error) => panic!("cannot read the commits of `{branch}` since `{base}`: {error}"),
    };
    let accepted = accepted_formats(&root);
    let mut used = BTreeSet::new();
    let refused: Vec<String> = commits
        .iter()
        .flat_map(|(hash, message)| refusals(&hash[..12], message, &accepted, &mut used))
        .collect();
    println!(
        "{} commit(s) of `{branch}` since its merge-base with `{base}`; accepted formats {accepted:?}",
        commits.len()
    );
    assert!(
        refused.is_empty(),
        "{} unaccepted completion claim(s) in commit messages:\n{}",
        refused.len(),
        refused.join("\n")
    );
}

#[test]
fn no_document_claims_an_unaccepted_completion() {
    let root = repository_root();
    let tree = WorkingTree::read(&root);
    let documents: Vec<&String> = tree
        .paths()
        .iter()
        .filter(|path| path.starts_with("docs/"))
        .collect();
    assert!(
        documents.len() >= 20,
        "only {} document(s) under docs/",
        documents.len()
    );
    let accepted = accepted_formats(&root);
    let mut used = BTreeSet::new();
    let mut refused = Vec::new();
    for path in &documents {
        let Ok(text) = std::fs::read_to_string(root.join(path)) else {
            continue;
        };
        refused.extend(refusals(path, &text, &accepted, &mut used));
    }
    for (index, (prefix, excerpt, _)) in EXCEPTIONS.iter().enumerate() {
        if !used.contains(&index) && !prefix.chars().all(|c| c.is_ascii_hexdigit()) {
            refused.push(format!(
                "exception {prefix} {excerpt:?} matched nothing and is stale"
            ));
        }
    }
    println!(
        "{} document(s) under docs/; accepted formats {accepted:?}",
        documents.len()
    );
    assert!(
        refused.is_empty(),
        "{} unaccepted completion claim(s) in documents:\n{}",
        refused.len(),
        refused.join("\n")
    );
}

#[test]
fn every_acceptance_record_is_well_formed() {
    let root = repository_root();
    let tree = WorkingTree::read(&root);
    let mut failures = Vec::new();
    let mut read = 0;
    for path in tree
        .paths()
        .iter()
        .filter(|path| path.starts_with(&format!("{ACCEPTANCE_DIRECTORY}/")))
    {
        let name = &path[ACCEPTANCE_DIRECTORY.len() + 1..];
        let Some(format) = name
            .strip_suffix(".md")
            .filter(|format| FORMATS.contains(format))
        else {
            failures.push(format!("{path} is not named after one of {FORMATS:?}"));
            continue;
        };
        let text = std::fs::read_to_string(root.join(path)).unwrap_or_default();
        if let Err(problem) = parse_record(&text, format) {
            failures.push(format!("{path}: {problem}"));
        }
        read += 1;
    }
    for (_, _, reason) in EXCEPTIONS {
        if reason.trim().is_empty() {
            failures.push("an exception states no reason".to_owned());
        }
    }
    println!("{read} acceptance record(s)");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_claim_reader_finds_claims_and_only_claims() {
    let needs = |text: &str| -> Vec<Vec<&'static str>> {
        claims(text)
            .into_iter()
            .map(|(formats, _)| formats.into_iter().collect())
            .collect()
    };
    assert_eq!(
        needs("Merge MJXOFF-177 (R22). Word is complete."),
        [["docx"]]
    );
    assert_eq!(
        needs("the parity ledger. PHASE R IS COMPLETE."),
        [["docx", "pptx", "xlsx"]]
    );
    assert_eq!(needs("Excel is complete for this loop"), [["xlsx"]]);
    assert_eq!(needs("**The pptx renderer is complete.**"), [["pptx"]]);
    assert_eq!(needs("Word and Excel are complete"), [["docx", "xlsx"]]);
    assert_eq!(
        needs("The renderer is now complete"),
        [["docx", "pptx", "xlsx"]]
    );
    for prose in [
        "The color resolver is complete.",
        "each word is complete",
        "The shape-outline workstream is complete.",
        "the Word table is completed later",
        "Handoff — PowerPoint images — COMPLETE",
        "Word is completely rewritten",
    ] {
        assert!(needs(prose).is_empty(), "read {prose:?} as a claim");
    }
}

#[test]
fn the_record_parser_accepts_the_defined_header_and_refuses_the_rest() {
    let record = |fields: &str| {
        format!("{RECORD_OPENING}\n{fields}\n{RECORD_CLOSING}\n\n# Word acceptance\n")
    };
    let good = "format: docx\ngate: RC46\ndate: 2026-10-01\napprover: A Person\nfixture: tests/fixtures/corporate.docx\nverdict: passed\nwindows-run: sitting 3, run 12";
    let parsed = parse_record(&record(good), "docx").expect("the defined header parses");
    assert_eq!(
        (parsed.format.as_str(), parsed.verdict.as_str()),
        ("docx", "passed")
    );
    let broken = [
        (
            good.replace("format: docx", "format: xlsx"),
            "states format",
        ),
        (good.replace("gate: RC46", "gate: RC45"), "states gate"),
        (
            good.replace("verdict: passed", "verdict: pending"),
            "verdict",
        ),
        (good.replace("date: 2026-10-01", "date: 1 October"), "date"),
        (
            good.replace("\nwindows-run: sitting 3, run 12", ""),
            "`windows-run` is missing",
        ),
        (
            good.replace("approver: A Person", "approver:"),
            "`approver` is empty",
        ),
        (format!("{good}\nverdict: passed"), "stated twice"),
        (format!("{good}\nnotes: fine"), "unknown header key"),
    ];
    for (fields, expected) in broken {
        let problem = parse_record(&record(&fields), "docx").expect_err(expected);
        assert!(
            problem.contains(expected),
            "{problem:?} does not mention {expected:?}"
        );
    }
    assert!(parse_record("# no header", "docx").is_err());
}

#[test]
fn a_missing_ref_is_a_reported_error_rather_than_a_panic() {
    let root = repository_root();
    let missing = "refs/heads/mjxoff-298-no-such-branch";
    assert_eq!(
        resolve(&root, missing),
        Err(HistoryError::MissingRef(missing.to_owned()))
    );
    let error = commits_since_base(&root, "HEAD", missing);
    assert!(
        matches!(
            error,
            Err(HistoryError::MissingRef(_)) | Err(HistoryError::Shallow)
        ),
        "{error:?}"
    );
    assert!(HistoryError::Shallow.to_string().contains("--unshallow"));
}
