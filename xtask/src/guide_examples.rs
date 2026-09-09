//! **The guide holds markers, not code.** (MJXOFF-254.)
//!
//! A guide example is *three real files a real toolchain runs* — a `cargo` example, a `pytest`
//! module and a `node --test` module — and the code blocks a reader sees in the `.md` are copies of
//! sentinel-delimited regions of those files, made by this module. `cargo run -p xtask --
//! guide-examples` writes them; `cargo run -p xtask -- guide-examples --check` writes nothing and
//! reports whether the committed blocks are what the sources say today;
//! `xtask/tests/guide_examples.rs` is the gate that makes the answer a test failure.
//!
//! # Why copying rather than generating
//!
//! The alternative — emit the Python and JavaScript from the Rust — needs a model of the binding
//! projection, and the projection is not mechanical: a `CellInput` becomes a `CellWrite`
//! constructor, an `ErrorCode` becomes a string on `.code`, a range argument becomes two numbers, a
//! `Format` accessor becomes a free function. A translator that is subtly wrong in that layer is
//! worse than none, which is `xtask/tests/facade_curation.rs`'s own verdict about parsing
//! signatures. Copying has no model to be wrong about: the block *is* the file.
//!
//! # Committed output, never a `build.rs`
//!
//! The same rule `CLAUDE.md` states for `mjx-ooxml-types`. The rendered pages are committed, this
//! command regenerates them, and the gate compares. Nothing runs at build time.
//!
//! # The shape of a marked block
//!
//! ````text
//! <!-- guide-example: saving_validates rust -->
//! ```rust
//! …the source file's region, verbatim…
//! ```
//! <!-- guide-example end -->
//! ````
//!
//! Everything between the two marker lines is rewritten from the source, so hand-editing a block is
//! an edit that the next `--check` reports and the next run discards.
//!
//! # The fourth marker form: a block that is Rust-only, and says which names make it so
//!
//! Three languages is the rule and it is the right default — it is what stops one of them quietly
//! falling behind. But a little of this facade **is Rust-only by decision**, and a block about it
//! can never have a Python or a JavaScript half. Before MJXOFF-261 the only way to express that was
//! to write no marker at all, which is indistinguishable from having forgotten, and forgetting is
//! exactly what the gate exists to catch.
//!
//! So such a block carries a marker of its own, and the marker names the **Rust symbols that make
//! the claim true**:
//!
//! ````text
//! <!-- guide-example: the_escape_hatches rust-only presentation_mut document_mut workbook_mut -->
//! ```rust
//! …the source file's region, verbatim…
//! ```
//! <!-- guide-example end -->
//! ````
//!
//! It renders exactly as a `rust` marker does — one block, in Rust. What it changes is what the
//! gate then demands, and the demand is **stronger** rather than weaker:
//!
//! * the example must have a Rust half and **no** Python or JavaScript half, so the exemption
//!   cannot be a place a binding half goes to be forgotten;
//! * every name it lists must appear in the region the block shows, so the reason is about *this*
//!   block rather than about the language in general;
//! * every name it lists must be reachable from **neither** binding, read out of the committed
//!   `.pyi` and the committed `#[wasm_bindgen]` declarations by [`crate::binding_surface`]. The day
//!   a binding projects one of them, the claim stops being true and the gate says so.
//!
//! That last one is the whole difference between this and a suppression. A marker that merely
//! turned a check off would be the nominal gate this repository keeps deleting; this one is a claim
//! the repository is asked to confirm on every run.
//!
//! The human sentence lives beside the block in the page's own prose, where a reader will meet it.
//! The marker carries the part a test can check.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

/// Opens a marker: the rest of the line is `<name> <language> -->`.
pub const MARKER_PREFIX: &str = "<!-- guide-example: ";

/// Ends a marker line.
pub const MARKER_SUFFIX: &str = " -->";

/// Closes a marked block. Everything between this and its opener is generated.
pub const MARKER_END: &str = "<!-- guide-example end -->";

/// The token that stands where a language would, on a block that is Rust-only by decision.
///
/// It is followed by one or more Rust symbols — the names that make the claim true. See this
/// module's header for what the gate then holds them to.
pub const RUST_ONLY_TOKEN: &str = "rust-only";

/// The sentinel that opens the copied region of a source file.
pub const REGION_START: &str = "guide-example:start";

/// The sentinel that closes it.
pub const REGION_END: &str = "guide-example:end";

/// The sentinel that opens a Rust half's *hidden* region, if it has one.
///
/// An example that starts from a file needs bytes, and reading a file is the caller's job rather
/// than this library's — every guide page says so, and `crates/mjx-ooxml/examples/build_a_deck.rs`
/// keeps its own `std::fs::read` outside the code the guide shows for exactly that reason. The
/// Python and JavaScript halves get that for free: anything above their sentinel is simply not in
/// the block. The Rust half cannot, because its block is *also* a doctest, and a doctest that names
/// `original` without binding it does not compile.
///
/// So a Rust half may carry a second, earlier region whose lines are emitted into the block as
/// rustdoc's hidden `#` lines: compiled and run, never shown. It is the same device the guide's
/// hand-written blocks already use for `fn main`, made available to a copied one.
///
/// Deliberately Rust-only. In the other two languages it would be a no-op — the lines are already
/// invisible — and `xtask/tests/guide_examples.rs` reports one there rather than ignoring it.
pub const PRELUDE_START: &str = "guide-example:prelude-start";

/// The sentinel that closes it.
///
/// Neither prelude sentinel contains [`REGION_START`] or [`REGION_END`] as a substring, so the two
/// pairs cannot be confused for one another by the scan below.
pub const PRELUDE_END: &str = "guide-example:prelude-end";

/// The hidden line a copied Rust region is wrapped in so the block is a runnable doctest.
///
/// It is hidden (`#`) rather than shown because it is scaffolding, not API: the reader sees the
/// calls, and `cargo test --doc -p mjx-ooxml` still compiles and runs them.
const RUST_DOCTEST_OPEN: &str = "# fn main() -> Result<(), Box<dyn std::error::Error>> {";

/// The closing half of [`RUST_DOCTEST_OPEN`].
const RUST_DOCTEST_CLOSE: &str = "# Ok(())\n# }";

/// Directories the page walk never descends into.
///
/// Build output, the git-ignored schema tree, installed packages and caches. A marker in any of
/// them would not be a guide page.
const SKIPPED_DIRECTORIES: [&str; 8] = [
    ".git",
    ".venv",
    "target",
    "References",
    "node_modules",
    "dist",
    "__pycache__",
    ".pytest_cache",
];

/// The three languages the API ships in, and where each one's half of an example lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Language {
    /// The facade itself: a `cargo` example, and the same code as a compiled doctest.
    Rust,
    /// `bindings/mjx-python`, run by `pytest`.
    Python,
    /// `bindings/mjx-wasm`, run by `node --test`.
    JavaScript,
}

impl Language {
    /// Every language, in the order a marked section presents them.
    pub const ALL: [Language; 3] = [Language::Rust, Language::Python, Language::JavaScript];

    /// How a marker names it, which is also the fence tag its block carries.
    ///
    /// `js` rather than `ts`: the file `node --test` runs is JavaScript, and nothing in this
    /// repository type-checks a TypeScript rendering of it.
    pub fn token(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::JavaScript => "js",
        }
    }

    /// The language a marker's token names, if it names one.
    pub fn from_token(token: &str) -> Option<Language> {
        Language::ALL
            .into_iter()
            .find(|language| language.token() == token)
    }

    /// The directory this language's halves live in, relative to the repository root.
    pub fn directory(self) -> &'static str {
        match self {
            Language::Rust => "crates/mjx-ooxml/examples",
            Language::Python => "bindings/mjx-python/tests/guide_examples",
            Language::JavaScript => "bindings/mjx-wasm/tests/node/guide_examples",
        }
    }

    /// The file name an example of this name has here.
    ///
    /// The Rust half shares a directory with the facade's walkthroughs, so it carries a prefix; the
    /// other two have a directory of their own and do not.
    pub fn file_name(self, name: &str) -> String {
        match self {
            Language::Rust => format!("guide_{name}.rs"),
            Language::Python => format!("{name}.py"),
            Language::JavaScript => format!("{name}.mjs"),
        }
    }

    /// The inverse: the example a file in [`Language::directory`] is a half of, if it is one.
    pub fn name_of(self, file: &str) -> Option<&str> {
        match self {
            Language::Rust => file.strip_suffix(".rs")?.strip_prefix("guide_"),
            Language::Python => file.strip_suffix(".py"),
            Language::JavaScript => file.strip_suffix(".mjs"),
        }
    }

    /// Where this language's half of `name` is, relative to the repository root.
    pub fn source_path(self, name: &str) -> String {
        format!("{}/{}", self.directory(), self.file_name(name))
    }

    /// The body of the fenced block, from the region copied out of the source file.
    ///
    /// Only Rust adds anything, and what it adds is hidden from the rendered page: the `fn main`
    /// the doctest needs, and — for an example that starts from bytes it did not author — the
    /// [`PRELUDE_START`] region that binds them. The other two languages hide their setup by
    /// leaving it above the sentinel, which is why they take no prelude.
    pub fn block_body(self, region: &str, prelude: Option<&str>) -> String {
        match self {
            Language::Rust => {
                let mut body = String::from(RUST_DOCTEST_OPEN);
                for line in prelude.into_iter().flat_map(str::lines) {
                    body.push('\n');
                    // `#` alone on a blank line, so a hidden line never leaves trailing space in
                    // the committed page.
                    body.push('#');
                    if !line.is_empty() {
                        body.push(' ');
                        body.push_str(line);
                    }
                }
                body.push('\n');
                body.push_str(region);
                body.push('\n');
                body.push_str(RUST_DOCTEST_CLOSE);
                body
            }
            Language::Python | Language::JavaScript => region.to_owned(),
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.token())
    }
}

/// One marked block found in a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marker {
    /// The example it copies, without any language's prefix or extension.
    pub name: String,
    /// Which of the three halves it shows.
    pub language: Language,
    /// The Rust symbols that make this block Rust-only, when it declares itself so.
    ///
    /// Empty on an ordinary marker, and never empty on a [`RUST_ONLY_TOKEN`] one — a claim with no
    /// names is refused where it is parsed, because a reason nothing can be compared against is
    /// the suppression this form exists not to be.
    pub rust_only: Vec<String>,
    /// The one-based line the marker opens on, so a failure can be navigated to.
    pub line: usize,
}

impl Marker {
    /// Whether this block declares itself Rust-only.
    #[must_use]
    pub fn is_rust_only(&self) -> bool {
        !self.rust_only.is_empty()
    }
}

/// A page whose marked blocks are not what its sources say.
#[derive(Clone, Debug)]
pub struct PageUpdate {
    /// The page, relative to the repository root.
    pub path: String,
    /// What is committed today.
    pub committed: String,
    /// What the sources say it should be.
    pub rendered: String,
}

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// The copied region of one source file: the lines between the two sentinels, dedented.
///
/// Errors rather than returning an empty region, because an example whose region is empty renders
/// an empty block and nothing else would notice.
pub fn region(source: &str, path: &str) -> Result<String> {
    between(source, path, REGION_START, REGION_END)?.ok_or_else(|| {
        anyhow!("{path}: expected exactly one `{REGION_START}` and one `{REGION_END}`, found none")
    })
}

/// The hidden region of a Rust half, when it has one.
///
/// `Ok(None)` is the ordinary answer — most examples need no setup. A file that opens the pair and
/// never closes it, or encloses nothing, is an error rather than a prelude quietly dropped: the
/// block would then compile against a binding that is not there and fail somewhere else entirely.
pub fn prelude(source: &str, path: &str) -> Result<Option<String>> {
    between(source, path, PRELUDE_START, PRELUDE_END)
}

/// The dedented lines between one pair of sentinels, or `None` when the file carries neither.
fn between(source: &str, path: &str, start: &str, end: &str) -> Result<Option<String>> {
    let opens: Vec<usize> = source
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(start))
        .map(|(index, _)| index)
        .collect();
    let closes: Vec<usize> = source
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(end))
        .map(|(index, _)| index)
        .collect();
    if opens.is_empty() && closes.is_empty() {
        return Ok(None);
    }
    if opens.len() != 1 || closes.len() != 1 {
        bail!(
            "{path}: expected exactly one `{start}` and one `{end}`, found {} and {}",
            opens.len(),
            closes.len()
        );
    }
    let (open, close) = (opens[0], closes[0]);
    if close <= open + 1 {
        bail!("{path}: `{start}` and `{end}` enclose no lines");
    }

    let mut lines: Vec<&str> = source.lines().collect::<Vec<_>>()[open + 1..close].to_vec();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        bail!("{path}: the region between `{start}` and `{end}` is blank");
    }

    let indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    Ok(Some(
        lines
            .iter()
            .map(|line| {
                if line.len() >= indent {
                    &line[indent..]
                } else {
                    ""
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
    ))
}

/// The marker one line opens, if it opens one.
///
/// A line that begins like a marker and is then malformed — no closing `-->`, no language, a
/// language that is not one of the three — is an error rather than a line quietly treated as prose.
/// Silence there is how a marker becomes a block nothing regenerates.
fn parse_marker_line(line: &str, page: &str, number: usize) -> Result<Option<Marker>> {
    let Some(rest) = line.trim_end().strip_prefix(MARKER_PREFIX) else {
        return Ok(None);
    };
    let rest = rest
        .strip_suffix(MARKER_SUFFIX)
        .ok_or_else(|| anyhow!("{page}:{number}: a marker line ends `{MARKER_SUFFIX}`"))?;
    let mut words = rest.split_whitespace();
    let name = words
        .next()
        .ok_or_else(|| anyhow!("{page}:{number}: a marker names an example"))?;
    let token = words
        .next()
        .ok_or_else(|| anyhow!("{page}:{number}: a marker names a language"))?;
    if token == RUST_ONLY_TOKEN {
        let rust_only: Vec<String> = words.map(str::to_owned).collect();
        if rust_only.is_empty() {
            bail!(
                "{page}:{number}: `{RUST_ONLY_TOKEN}` names the Rust symbols that make the block \
                 Rust-only, and at least one of them. A claim with no names is a suppression."
            );
        }
        return Ok(Some(Marker {
            name: name.to_owned(),
            language: Language::Rust,
            rust_only,
            line: number,
        }));
    }
    if let Some(extra) = words.next() {
        bail!("{page}:{number}: unexpected {extra:?} after the language");
    }
    let language = Language::from_token(token)
        .ok_or_else(|| anyhow!("{page}:{number}: {token:?} is not one of the three languages"))?;
    Ok(Some(Marker {
        name: name.to_owned(),
        language,
        rust_only: Vec::new(),
        line: number,
    }))
}

/// Every marker in one page, in the order they appear.
///
/// A marker with no `<!-- guide-example end -->` after it, or one whose language is not one of the
/// three, is an error rather than a marker that is quietly ignored.
pub fn markers_in(text: &str, page: &str) -> Result<Vec<Marker>> {
    let lines: Vec<&str> = text.lines().collect();
    let mut markers = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(marker) = parse_marker_line(line, page, index + 1)? else {
            continue;
        };
        if !lines[index + 1..]
            .iter()
            .any(|later| later.trim() == MARKER_END)
        {
            bail!(
                "{page}:{}: this marker is never closed by `{MARKER_END}`",
                marker.line
            );
        }
        markers.push(marker);
    }
    Ok(markers)
}

/// One page, rewritten: every marked block replaced by a copy of its source's region.
///
/// Text outside a marked block is untouched, so a page is prose the generator does not own with
/// windows into files it does.
pub fn rewrite_page(root: &Path, page: &str, text: &str) -> Result<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut output: Vec<String> = Vec::with_capacity(lines.len());
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let Some(marker) = parse_marker_line(line, page, index + 1)? else {
            output.push(line.to_owned());
            index += 1;
            continue;
        };
        // Everything up to and including the closing marker belongs to the generator.
        let end = lines[index + 1..]
            .iter()
            .position(|later| later.trim() == MARKER_END)
            .map(|offset| index + 1 + offset)
            .ok_or_else(|| anyhow!("{page}:{}: unclosed marker", marker.line))?;

        let source_path = marker.language.source_path(&marker.name);
        let source = std::fs::read_to_string(root.join(&source_path))
            .with_context(|| format!("{page}:{}: reading {source_path}", marker.line))?;
        let extracted = region(&source, &source_path)
            .with_context(|| format!("{page}:{}: extracting {source_path}", marker.line))?;
        let hidden = prelude(&source, &source_path)
            .with_context(|| format!("{page}:{}: extracting {source_path}", marker.line))?;
        let body = marker.language.block_body(&extracted, hidden.as_deref());

        output.push(line.to_owned());
        output.push(format!("```{}", marker.language.token()));
        output.extend(body.lines().map(str::to_owned));
        output.push("```".to_owned());
        output.push(lines[end].to_owned());
        index = end + 1;
    }
    let mut rendered = output.join("\n");
    if text.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}

/// Every `.md` file under the repository that the page walk considers, relative to the root.
///
/// The walk is over the whole repository rather than one guide directory on purpose: a marker
/// placed in a page nobody thought to list would otherwise be a block nothing regenerates.
pub fn pages(root: &Path) -> Result<Vec<String>> {
    let mut found = Vec::new();
    walk(root, root, &mut found)?;
    found.sort();
    Ok(found)
}

/// The recursive half of [`pages`].
fn walk(root: &Path, directory: &Path, found: &mut Vec<String>) -> Result<()> {
    let entries =
        std::fs::read_dir(directory).with_context(|| format!("reading {}", directory.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading {}", directory.display()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let file_type = entry
            .file_type()
            .with_context(|| format!("stat {}", path.display()))?;
        if file_type.is_dir() {
            if SKIPPED_DIRECTORIES.contains(&name.as_str()) {
                continue;
            }
            walk(root, &path, found)?;
        } else if file_type.is_file() && name.ends_with(".md") {
            let relative = path
                .strip_prefix(root)
                .map(|relative| relative.to_string_lossy().into_owned())
                .unwrap_or(name);
            found.push(relative);
        }
    }
    Ok(())
}

/// Every marker in the repository, page by page.
pub fn all_markers(root: &Path) -> Result<Vec<(String, Marker)>> {
    let mut markers = Vec::new();
    for page in pages(root)? {
        let text =
            std::fs::read_to_string(root.join(&page)).with_context(|| format!("reading {page}"))?;
        if !text.contains(MARKER_PREFIX) {
            continue;
        }
        for marker in markers_in(&text, &page)? {
            markers.push((page.clone(), marker));
        }
    }
    Ok(markers)
}

/// Every example a marker declares Rust-only, with the names it declares make it so.
///
/// Derived from the markers, so there is no second list anywhere saying which examples are
/// Rust-only — the marker beside the block is the only statement of it.
pub fn rust_only_examples(root: &Path) -> Result<BTreeMap<String, Vec<String>>> {
    let mut declared: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (_, marker) in all_markers(root)? {
        if marker.is_rust_only() {
            declared.insert(marker.name, marker.rust_only);
        }
    }
    Ok(declared)
}

/// The examples a directory holds a half of, derived from the filesystem rather than listed.
pub fn halves_present(root: &Path, language: Language) -> Result<BTreeSet<String>> {
    let directory = root.join(language.directory());
    let mut names = BTreeSet::new();
    let entries = std::fs::read_dir(&directory)
        .with_context(|| format!("reading {}", directory.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading {}", directory.display()))?;
        if !entry
            .file_type()
            .with_context(|| format!("stat {}", entry.path().display()))?
            .is_file()
        {
            continue;
        }
        let file = entry.file_name().to_string_lossy().into_owned();
        if let Some(name) = language.name_of(&file) {
            names.insert(name.to_owned());
        }
    }
    Ok(names)
}

/// Every page whose committed blocks differ from what its sources say today.
///
/// An empty result is the whole of the content-equality property: each committed block is byte for
/// byte the region of the file a harness runs.
pub fn plan(root: &Path) -> Result<Vec<PageUpdate>> {
    let mut updates = Vec::new();
    for page in pages(root)? {
        let committed =
            std::fs::read_to_string(root.join(&page)).with_context(|| format!("reading {page}"))?;
        if !committed.contains(MARKER_PREFIX) {
            continue;
        }
        let rendered = rewrite_page(root, &page, &committed)?;
        if rendered != committed {
            updates.push(PageUpdate {
                path: page,
                committed,
                rendered,
            });
        }
    }
    Ok(updates)
}

/// Copy every source region into the page that marks it, and say what changed.
pub fn run() -> Result<()> {
    let root = repository_root();
    let markers = all_markers(&root)?;
    let updates = plan(&root)?;
    for update in &updates {
        std::fs::write(root.join(&update.path), &update.rendered)
            .with_context(|| format!("writing {}", update.path))?;
        println!("rewrote {}", update.path);
    }
    let examples: BTreeSet<&str> = markers
        .iter()
        .map(|(_, marker)| marker.name.as_str())
        .collect();
    let rust_only = markers
        .iter()
        .filter(|(_, marker)| marker.is_rust_only())
        .count();
    println!(
        "{} marker(s) over {} example(s), {rust_only} of them declared Rust-only; {} page(s) \
         rewritten",
        markers.len(),
        examples.len(),
        updates.len()
    );
    Ok(())
}

/// The same comparison, writing nothing.
pub fn check() -> Result<()> {
    let root = repository_root();
    let markers = all_markers(&root)?;
    let updates = plan(&root)?;
    let examples: BTreeSet<&str> = markers
        .iter()
        .map(|(_, marker)| marker.name.as_str())
        .collect();
    let rust_only = markers
        .iter()
        .filter(|(_, marker)| marker.is_rust_only())
        .count();
    println!(
        "{} marker(s) over {} example(s), {rust_only} of them declared Rust-only",
        markers.len(),
        examples.len()
    );
    if updates.is_empty() {
        println!("every marked block is a current copy of its source");
        return Ok(());
    }
    bail!(
        "{} page(s) hold a block that is not what its source says:\n  {}\n\nRun `cargo run -p \
         xtask -- guide-examples` to copy the sources in again.",
        updates.len(),
        updates
            .iter()
            .map(|update| update.path.clone())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
