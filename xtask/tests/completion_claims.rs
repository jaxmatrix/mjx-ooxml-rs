//! The completion-claim gate (MJXOFF-298): a commit message from HEAD back to its merge-base with `main`, or any Markdown file in the working tree, that says a format or the renderer is complete needs a passed RC46 acceptance record for it.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use xtask::repository_files::WorkingTree;

// The commit whose history back to its merge-base with the base is read, overridable for a scratch branch.
const BRANCH_VARIABLE: &str = "MJX_COMPLETION_CLAIMS_BRANCH";
const DEFAULT_HEAD: &str = "HEAD";
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

// Approver values that name no person, compared in any case: a generated stamp is not a human review.
const APPROVER_PLACEHOLDERS: [&str; 7] = ["tbd", "tba", "generator", "n/a", "none", "unknown", "-"];

// Windows-run values that name no run, compared in any case.
const RUN_PLACEHOLDERS: [&str; 9] = [
    "tbd", "tba", "n/a", "none", "pending", "unknown", "todo", "-", "?",
];

// The words a claim's subject is spelled with, formats and products first, and the formats a claim naming each needs accepted.
const SUBJECTS: [(&str, &[&str]); 9] = [
    ("pptx", &["pptx"]),
    ("PowerPoint", &["pptx"]),
    ("docx", &["docx"]),
    ("Word", &["docx"]),
    ("xlsx", &["xlsx"]),
    ("Excel", &["xlsx"]),
    ("renderer", &["pptx", "docx", "xlsx"]),
    ("Phase R", &["pptx", "docx", "xlsx"]),
    ("all three formats", &["pptx", "docx", "xlsx"]),
];

// How many leading SUBJECTS entries are format or product names, which alone may qualify a subject noun.
const PRODUCT_SUBJECTS: usize = 6;

// The predicates a completion claim is written with, matched in any case.
const PREDICATES: [&str; 10] = [
    " is complete",
    " is now complete",
    " are complete",
    " is done",
    " is finished",
    " is fully implemented",
    " is fully complete",
    " is feature-complete",
    " has been completed",
    " have been completed",
];

// Nouns that may follow a format name in a claim's subject, as in "the pptx box model is complete".
const SUBJECT_NOUNS: [&str; 5] = ["renderer", "rendering", "support", "parity", "box model"];

// Claims that are not a present completion claim: a document path or commit hash prefix, an excerpt of the claim, and why.
const EXCEPTIONS: [(&str, &str, &str); 2] = [
    (
        "6fa491c0d690",
        "or the renderer has been completed needs",
        "the commit that added this gate, describing the claims it refuses",
    ),
    (
        "CHANGELOG.md",
        "Word is complete for view.",
        "a historical release note for MJXOFF-177 (R22), written before the RC46 acceptance record existed",
    ),
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits one level below the repository root")
        .to_path_buf()
}

#[derive(PartialEq, Eq)]
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

// A test returning this error prints it through `Debug`, so `Debug` is the readable message.
impl fmt::Debug for HistoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
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

// Each commit reachable from `head` after its merge-base with `base`, as hash and message.
fn commits_since_base(
    root: &Path,
    head: &str,
    base: &str,
) -> Result<Vec<(String, String)>, HistoryError> {
    if git(root, &["rev-parse", "--is-shallow-repository"])?.trim() == "true" {
        return Err(HistoryError::Shallow);
    }
    let head = resolve(root, head)?;
    let base = resolve(root, base)?;
    let merge_base = git(root, &["merge-base", &base, &head])?;
    let range = format!("{}..{head}", merge_base.trim());
    let log = git(root, &["log", "--format=%H%x00%B%x1e", &range])?;
    Ok(log
        .split('\u{1e}')
        .filter_map(|entry| entry.trim_start().split_once('\0'))
        .map(|(hash, message)| (hash.to_owned(), message.to_owned()))
        .collect())
}

fn environment_or(variable: &str, default: &str) -> String {
    std::env::var(variable).unwrap_or_else(|_| default.to_owned())
}

// The head and base the gate reads between: HEAD and `main` unless the environment overrides them.
fn gate_range() -> (String, String) {
    (
        environment_or(BRANCH_VARIABLE, DEFAULT_HEAD),
        environment_or(BASE_VARIABLE, DEFAULT_BASE),
    )
}

// The commits the gate reads, from its range.
fn commits_for_gate(root: &Path) -> Result<Vec<(String, String)>, HistoryError> {
    let (head, base) = gate_range();
    commits_since_base(root, &head, &base)
}

fn trim_word(word: &str) -> &str {
    let word = word.trim_matches(|c: char| "*`_\"'()[]:;.,!?—".contains(c));
    word.strip_suffix("'s").unwrap_or(word)
}

// Whether `words` spell `subject`: format and product names exactly or in capitals, other subjects in any case.
fn spells(words: &str, subject: &str) -> bool {
    let words = words.trim_start_matches('.');
    if subject.chars().next().is_some_and(char::is_uppercase) {
        words == subject || words == subject.to_uppercase()
    } else {
        words.eq_ignore_ascii_case(subject)
    }
}

// A text's paragraphs with hard-wrapped lines joined, split at blank lines, with block-quote markers dropped.
fn joined_paragraphs(text: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in text.lines() {
        let line = line.trim_start().trim_start_matches('>').trim();
        if line.is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join(" "));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }
    paragraphs
}

// How many words `phrase` is.
fn word_count(phrase: &str) -> usize {
    phrase.split(' ').count()
}

// The formats a claim's subject needs, read backwards from the word before its predicate.
fn subject_formats(words: &[&str]) -> BTreeSet<&'static str> {
    let window = |from: usize, to: usize| words[from..to].join(" ");
    let mut formats = BTreeSet::new();
    let mut index = words.len();
    // A subject noun may close the subject, as long as a format or product name precedes it.
    let mut noun = false;
    while index > 0 {
        if !noun && formats.is_empty() {
            let qualified_noun = SUBJECT_NOUNS
                .iter()
                .map(|noun| word_count(noun))
                .find(|&count| {
                    index > count
                        && SUBJECT_NOUNS
                            .iter()
                            .any(|noun| window(index - count, index).eq_ignore_ascii_case(noun))
                        && SUBJECTS[..PRODUCT_SUBJECTS]
                            .iter()
                            .any(|(subject, _)| spells(words[index - count - 1], subject))
                });
            if let Some(count) = qualified_noun {
                noun = true;
                index -= count;
                continue;
            }
        }
        let subject = SUBJECTS
            .iter()
            .filter(|(subject, _)| {
                let count = word_count(subject);
                index >= count && spells(&window(index - count, index), subject)
            })
            .max_by_key(|(subject, _)| word_count(subject));
        if let Some((subject, needs)) = subject {
            formats.extend(needs.iter().copied());
            index -= word_count(subject);
        } else if !formats.is_empty() && ["and", "&", "or"].contains(&words[index - 1]) {
            index -= 1;
        } else {
            break;
        }
    }
    formats
}

// The sentence of `paragraph` around byte `at`.
fn sentence_around(paragraph: &str, at: usize) -> &str {
    let is_end = |(index, byte): (usize, &u8)| {
        matches!(byte, b'.' | b'?' | b'!') && paragraph.as_bytes().get(index + 1) == Some(&b' ')
    };
    let bytes = paragraph.as_bytes();
    let start = bytes[..at]
        .iter()
        .enumerate()
        .rev()
        .find(|&entry| is_end(entry))
        .map_or(0, |(index, _)| index + 2);
    let end = bytes[at..]
        .iter()
        .enumerate()
        .map(|(offset, byte)| (at + offset, byte))
        .find(|&entry| is_end(entry))
        .map_or(paragraph.len(), |(index, _)| index + 1);
    paragraph[start..end].trim()
}

// The formats each completion claim in `text` needs accepted, with the sentence it was read from.
fn claims(text: &str) -> Vec<(BTreeSet<&'static str>, String)> {
    let mut found = Vec::new();
    for paragraph in joined_paragraphs(text) {
        let lower = paragraph.to_ascii_lowercase();
        for predicate in PREDICATES {
            for (at, _) in lower.match_indices(predicate) {
                let after = lower.as_bytes().get(at + predicate.len());
                if after.is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'-') {
                    continue;
                }
                let words: Vec<&str> = paragraph[..at]
                    .split_whitespace()
                    .map(trim_word)
                    .filter(|word| !word.is_empty())
                    .collect();
                let formats = subject_formats(&words);
                if !formats.is_empty() {
                    found.push((formats, sentence_around(&paragraph, at).to_owned()));
                }
            }
        }
    }
    found
}

// Today's date in UTC as year, month and day.
fn today() -> (i32, u32, u32) {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    civil_from_days((seconds / 86_400) as i64)
}

// The proleptic Gregorian date `days` after 1970-01-01, by Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year as i32, month as u32, day as u32)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

// A `YYYY-MM-DD` value as a real calendar date, or `None`.
fn calendar_date(value: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = value.split('-').collect();
    let shaped = parts.len() == 3
        && [4, 2, 2].iter().zip(&parts).all(|(length, part)| {
            part.len() == *length && part.bytes().all(|byte| byte.is_ascii_digit())
        });
    if !shaped {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    ((1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day))
        .then_some((year, month, day))
}

// What a record is checked against beyond its own text: today's date and whether a fixture path exists.
struct RecordChecks<'a> {
    today: (i32, u32, u32),
    fixture_exists: &'a dyn Fn(&str) -> bool,
}

#[derive(Debug)]
struct Record {
    format: String,
    verdict: String,
}

fn is_placeholder(value: &str, placeholders: &[&str]) -> bool {
    placeholders
        .iter()
        .any(|placeholder| value.eq_ignore_ascii_case(placeholder))
}

// Parses an acceptance record for `format`, refusing any header the format definition above does not allow.
fn parse_record(text: &str, format: &str, checks: &RecordChecks<'_>) -> Result<Record, String> {
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
    let Some(stated) = calendar_date(date) else {
        return Err(format!(
            "date `{date}` is not a real calendar date written YYYY-MM-DD"
        ));
    };
    if stated > checks.today {
        let (year, month, day) = checks.today;
        return Err(format!(
            "date `{date}` is in the future; today is {year:04}-{month:02}-{day:02}"
        ));
    }
    let approver = field("approver")?;
    if is_placeholder(approver, &APPROVER_PLACEHOLDERS) {
        return Err(format!(
            "approver `{approver}` is a placeholder, not a person"
        ));
    }
    let fixture = field("fixture")?;
    if !(checks.fixture_exists)(fixture) {
        return Err(format!(
            "fixture `{fixture}` is not a path in the working tree"
        ));
    }
    let run = field("windows-run")?;
    if is_placeholder(run, &RUN_PLACEHOLDERS) {
        return Err(format!(
            "windows-run `{run}` is a placeholder, not a Windows run"
        ));
    }
    Ok(Record {
        format: format.to_owned(),
        verdict: verdict.to_owned(),
    })
}

// The formats whose acceptance record exists, parses, and records a passed gate.
fn accepted_formats(root: &Path, checks: &RecordChecks<'_>) -> BTreeSet<String> {
    FORMATS
        .iter()
        .filter_map(|format| {
            let text = std::fs::read_to_string(
                root.join(ACCEPTANCE_DIRECTORY).join(format!("{format}.md")),
            )
            .ok()?;
            parse_record(&text, format, checks).ok()
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
            let names_source = if prefix.chars().all(|c| c.is_ascii_hexdigit()) {
                source.starts_with(prefix)
            } else {
                source == *prefix
            };
            names_source && sentence.contains(excerpt)
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

// The Markdown documents a claim is read from: every `*.md` in the working tree outside `References/` and `target/`.
fn documents(paths: &[String]) -> Vec<&String> {
    paths
        .iter()
        .filter(|path| {
            path.ends_with(".md")
                && !path.starts_with("References/")
                && !path.starts_with("target/")
        })
        .collect()
}

// What one run of the gate read and refused, and which exceptions it used.
struct Report {
    commits: usize,
    documents: usize,
    accepted: BTreeSet<String>,
    refused: Vec<String>,
    used: BTreeSet<usize>,
}

// Reads the commits from `head` back to its merge-base with `base`, then every document in `paths`, refusing unaccepted claims in both.
fn scan(root: &Path, head: &str, base: &str, paths: &[String]) -> Result<Report, HistoryError> {
    let commits = commits_since_base(root, head, base)?;
    let fixture_exists = |fixture: &str| {
        paths
            .binary_search_by(|path| path.as_str().cmp(fixture))
            .is_ok()
    };
    let checks = RecordChecks {
        today: today(),
        fixture_exists: &fixture_exists,
    };
    let accepted = accepted_formats(root, &checks);
    let mut used = BTreeSet::new();
    let mut refused = Vec::new();
    for (hash, message) in &commits {
        refused.extend(refusals(&hash[..12], message, &accepted, &mut used));
    }
    let documents = documents(paths);
    for path in &documents {
        if let Ok(text) = std::fs::read_to_string(root.join(path)) {
            refused.extend(refusals(path, &text, &accepted, &mut used));
        }
    }
    Ok(Report {
        commits: commits.len(),
        documents: documents.len(),
        accepted,
        refused,
        used,
    })
}

#[test]
fn no_commit_or_document_claims_an_unaccepted_completion() -> Result<(), HistoryError> {
    let root = repository_root();
    let tree = WorkingTree::read(&root);
    let (head, base) = gate_range();
    let report = scan(&root, &head, &base, tree.paths())?;
    let mut refused = report.refused;
    assert!(
        report.documents >= 100,
        "only {} Markdown document(s) were read",
        report.documents
    );
    for (index, (prefix, excerpt, _)) in EXCEPTIONS.iter().enumerate() {
        if !report.used.contains(&index) && !prefix.chars().all(|c| c.is_ascii_hexdigit()) {
            refused.push(format!(
                "exception {prefix} {excerpt:?} matched nothing and is stale"
            ));
        }
    }
    println!(
        "{} commit(s) from `{head}` back to its merge-base with `{base}`, {} Markdown document(s); accepted formats {:?}",
        report.commits, report.documents, report.accepted
    );
    assert!(
        refused.is_empty(),
        "{} unaccepted completion claim(s):\n{}",
        refused.len(),
        refused.join("\n")
    );
    Ok(())
}

#[test]
fn every_acceptance_record_is_well_formed() {
    let root = repository_root();
    let tree = WorkingTree::read(&root);
    let paths = tree.paths();
    let fixture_exists = |fixture: &str| {
        paths
            .binary_search_by(|path| path.as_str().cmp(fixture))
            .is_ok()
    };
    let checks = RecordChecks {
        today: today(),
        fixture_exists: &fixture_exists,
    };
    let mut failures = Vec::new();
    let mut read = 0;
    for path in paths
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
        if let Err(problem) = parse_record(&text, format, &checks) {
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

// A synthetic record's checks: 2026-09-16, and only the corporate Word fixture exists.
fn corporate_fixture_only(path: &str) -> bool {
    path == "tests/fixtures/corporate.docx"
}

const SYNTHETIC_CHECKS: RecordChecks<'static> = RecordChecks {
    today: (2026, 9, 16),
    fixture_exists: &corporate_fixture_only,
};

#[test]
fn today_is_a_calendar_date() {
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(20_712), (2026, 9, 16));
    assert_eq!(civil_from_days(19_782), (2024, 2, 29));
    let (year, month, day) = today();
    assert!(year >= 2026 && calendar_date(&format!("{year:04}-{month:02}-{day:02}")).is_some());
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
    let good = "format: docx\ngate: RC46\ndate: 2026-09-01\napprover: A Person\nfixture: tests/fixtures/corporate.docx\nverdict: passed\nwindows-run: sitting 3, run 12";
    let parsed =
        parse_record(&record(good), "docx", &SYNTHETIC_CHECKS).expect("the defined header parses");
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
        (
            good.replace("date: 2026-09-01", "date: 1 September"),
            "date",
        ),
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
        let problem =
            parse_record(&record(&fields), "docx", &SYNTHETIC_CHECKS).expect_err(expected);
        assert!(
            problem.contains(expected),
            "{problem:?} does not mention {expected:?}"
        );
    }
    assert!(parse_record("# no header", "docx", &SYNTHETIC_CHECKS).is_err());
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

fn formats_claimed(text: &str) -> Vec<Vec<&'static str>> {
    claims(text)
        .into_iter()
        .map(|(formats, _)| formats.into_iter().collect())
        .collect()
}

const ALL: [&str; 3] = ["docx", "pptx", "xlsx"];

macro_rules! wording {
    ($($name:ident: $text:expr => $formats:expr;)*) => {
        $(
            #[test]
            fn $name() {
                let expected: Vec<Vec<&str>> = vec![$formats.to_vec()];
                assert_eq!(formats_claimed($text), expected, "{:?}", $text);
            }
        )*
    };
}

wording! {
    the_wording_is_done: "Word is done." => ["docx"];
    the_wording_is_finished: "PowerPoint is finished for this loop" => ["pptx"];
    the_wording_is_fully_implemented: "Excel is fully implemented." => ["xlsx"];
    the_wording_is_fully_complete: "The xlsx renderer is fully complete." => ["xlsx"];
    the_wording_is_feature_complete: "Word is feature-complete." => ["docx"];
    the_wording_has_been_completed: "The renderer has been completed." => ALL;
    the_wording_format_parity_is_complete: "pptx parity is complete" => ["pptx"];
    the_wording_product_parity_is_complete: "PowerPoint parity is complete." => ["pptx"];
    the_wording_all_three_formats_are_complete: "All three formats are complete." => ALL;
    the_wording_the_format_box_model_is_complete: "The docx box model is complete." => ["docx"];
    the_wording_the_format_renderer_is_complete: "the Excel renderer is complete" => ["xlsx"];
    the_wording_in_capitals: "WORD IS DONE" => ["docx"];
    the_wording_in_title_case: "The PPTX Box Model Is Complete" => ["pptx"];
    the_wording_in_mixed_case: "Excel Has Been Completed" => ["xlsx"];
}

#[test]
fn a_subject_wrapped_onto_the_previous_line_is_read() {
    assert_eq!(
        formats_claimed("Merge the corporate fixture. The pptx\nrenderer is complete."),
        [["pptx"]]
    );
    assert_eq!(
        formats_claimed("A long line that ends with Word\nand Excel are complete."),
        [["docx", "xlsx"]]
    );
    assert_eq!(formats_claimed("> Phase\n> R is complete"), [ALL]);
    assert!(formats_claimed("A list of Word\n\nis complete, a new paragraph").is_empty());
}

#[test]
fn the_document_corpus_is_every_markdown_file_in_the_working_tree() {
    let tree = WorkingTree::read(&repository_root());
    let documents = documents(tree.paths());
    for expected in ["CHANGELOG.md", "README.md", "PLAN.md", "CLAUDE.md"] {
        assert!(
            documents.iter().any(|path| *path == expected),
            "{expected} is not in the corpus"
        );
    }
    assert!(documents.iter().any(|path| path.starts_with("ui/")));
    assert!(documents
        .iter()
        .any(|path| path.starts_with("crates/") && path.contains("/docs/")));
    assert!(documents.iter().all(|path| path.ends_with(".md")
        && !path.starts_with("References/")
        && !path.starts_with("target/")));
    for (prefix, excerpt, reason) in EXCEPTIONS {
        assert!(
            excerpt.len() >= 12 && !prefix.ends_with('/') && !reason.trim().is_empty(),
            "exception {prefix} {excerpt:?} is not a path-plus-excerpt exception with a reason"
        );
    }
}

// A scratch repository under the test target directory, with `main` holding one commit.
fn scratch_repository(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("completion-claims-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("creating a scratch repository");
    run_git(&root, &["init", "--quiet", "--initial-branch", "main"]);
    std::fs::write(root.join("README.md"), "A scratch repository.\n").expect("writing README.md");
    run_git(&root, &["add", "README.md"]);
    run_git(&root, &["commit", "--quiet", "--message", "Start."]);
    root
}

fn run_git(root: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "user.name=Scratch",
            "-c",
            "user.email=scratch@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(arguments)
        .output()
        .expect("running git");
    assert!(
        status.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

#[test]
fn commits_are_read_from_head_back_to_its_merge_base_with_main() {
    let root = scratch_repository("head");
    run_git(&root, &["checkout", "--quiet", "-b", "some-feature"]);
    run_git(
        &root,
        &[
            "commit",
            "--quiet",
            "--allow-empty",
            "--message",
            "Word is complete.",
        ],
    );
    run_git(&root, &["checkout", "--quiet", "--detach"]);
    let commits = commits_for_gate(&root).map_err(|error| error.to_string());
    assert!(
        commits
            .as_ref()
            .is_ok_and(|commits| commits.len() == 1 && commits[0].1.contains("Word is complete.")),
        "{commits:?}"
    );
    let shallow = Path::new(env!("CARGO_TARGET_TMPDIR")).join("completion-claims-shallow");
    let _ = std::fs::remove_dir_all(&shallow);
    let source = format!("file://{}", root.display());
    let clone = Command::new("git")
        .args([
            "clone",
            "--quiet",
            "--depth",
            "1",
            "--branch",
            "some-feature",
            &source,
        ])
        .arg(&shallow)
        .output()
        .expect("running git clone");
    assert!(
        clone.status.success(),
        "{}",
        String::from_utf8_lossy(&clone.stderr)
    );
    assert_eq!(commits_for_gate(&shallow), Err(HistoryError::Shallow));
}

#[test]
fn an_empty_range_still_scans_the_documents() {
    let root = scratch_repository("empty");
    std::fs::create_dir_all(root.join("docs")).expect("creating docs/");
    std::fs::write(root.join("docs/status.md"), "Word is complete.\n").expect("writing a claim");
    let paths = ["README.md".to_owned(), "docs/status.md".to_owned()];
    let report = scan(&root, "HEAD", "main", &paths).map_err(|error| error.to_string());
    assert!(
        report.as_ref().is_ok_and(|report| report.commits == 0
            && report.documents >= 1
            && report
                .refused
                .iter()
                .any(|refusal| refusal.starts_with("docs/status.md"))),
        "{:?}",
        report.map(|report| (report.commits, report.documents, report.refused))
    );
}

#[test]
fn a_malformed_record_is_refused_field_by_field() {
    let fixture_exists = |path: &str| path == "tests/fixtures/corporate.docx";
    let checks = RecordChecks {
        today: (2026, 9, 16),
        fixture_exists: &fixture_exists,
    };
    let record = |fields: &str| format!("{RECORD_OPENING}\n{fields}\n{RECORD_CLOSING}\n");
    let good = "format: docx\ngate: RC46\ndate: 2026-09-01\napprover: A Person\nfixture: tests/fixtures/corporate.docx\nverdict: passed\nwindows-run: sitting 3, run 12";
    let mut failures = Vec::new();
    for date in ["2024-02-29", "2026-09-16"] {
        let fields = good.replace("date: 2026-09-01", &format!("date: {date}"));
        if let Err(problem) = parse_record(&record(&fields), "docx", &checks) {
            failures.push(format!("refused the valid date {date}: {problem}"));
        }
    }
    let mut broken: Vec<(String, &str)> = vec![
        (good.replace("date: 2026-09-01", "date: 2026-02-30"), "date"),
        (good.replace("date: 2026-09-01", "date: 2025-02-29"), "date"),
        (good.replace("date: 2026-09-01", "date: 2026-04-31"), "date"),
        (
            good.replace("date: 2026-09-01", "date: 2026-09-17"),
            "future",
        ),
        (
            good.replace("date: 2026-09-01", "date: 2031-01-01"),
            "future",
        ),
        (
            good.replace(
                "fixture: tests/fixtures/corporate.docx",
                "fixture: tests/fixtures/missing.docx",
            ),
            "fixture",
        ),
    ];
    for approver in [
        "TBD",
        "generator",
        "n/a",
        "none",
        "unknown",
        "Unknown",
        "tbd",
    ] {
        broken.push((
            good.replace("approver: A Person", &format!("approver: {approver}")),
            "approver",
        ));
    }
    for run in ["TBD", "n/a", "none", "pending", "unknown", "-", "?"] {
        broken.push((
            good.replace(
                "windows-run: sitting 3, run 12",
                &format!("windows-run: {run}"),
            ),
            "windows-run",
        ));
    }
    for (fields, expected) in &broken {
        match parse_record(&record(fields), "docx", &checks) {
            Ok(_) => failures.push(format!(
                "accepted a malformed record expecting {expected:?}:\n{fields}"
            )),
            Err(problem) if !problem.contains(expected) => {
                failures.push(format!("{problem:?} does not mention {expected:?}"))
            }
            Err(_) => {}
        }
    }
    assert!(
        failures.is_empty(),
        "{} problem(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
