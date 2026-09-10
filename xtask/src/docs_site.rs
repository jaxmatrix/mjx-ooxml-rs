//! **The user guide, rendered for the three languages it is written in.** (MJXOFF-281.)
//!
//! The facade guide's code blocks are already provably identical across Rust, Python and
//! JavaScript — `crate::guide_examples` is what makes that a test failure rather than a habit. That
//! guarantee reaches nobody outside Rust, because the guide renders **only in rustdoc**, which is
//! the one place a `pip install` or an `npm install` user never looks.
//!
//! `cargo run -p xtask -- docs-site` reads the committed guide markdown and writes a Docusaurus
//! content tree under `site/`, with each example's three halves as three tabs.
//!
//! # The output is generated and git-ignored
//!
//! The precedent is `bindings/mjx-wasm/npm/README.md`: generated, ignored, and its `package.json`
//! committed beside it. So this module writes into a directory `site/.gitignore` covers, and
//! `xtask/tests/docs_site.rs` asserts **properties of the generation from committed sources**
//! rather than comparing against a committed rendering. There is nothing committed to compare to,
//! and there should not be — a committed copy of a derived tree is a second source of truth.
//!
//! # Why the generator is here rather than in the site's own JavaScript
//!
//! Two of the transformations below need facts that live in Rust source:
//!
//! * the **page set and its reading order** come from the `include_str!` graph in
//!   `crates/mjx-ooxml/src/guide.rs` — the same list rustdoc renders from, so a page added there is
//!   on the site and a page added only to the site cannot exist; and
//! * the **shortcut link vocabulary** comes from that file's `guide_vocabulary!` macro, which is
//!   the set of names an intra-doc link on a facade guide page is allowed to resolve against.
//!
//! A JavaScript re-implementation of either would be the second parser this workspace keeps
//! refusing — `xtask/tests/binding_projection.rs`'s header states the reason and
//! `crate::guide_examples`'s states it again. This module goes further and reuses
//! [`crate::guide_examples::parse_marker_line`] itself, so the marker syntax has exactly one
//! reader.
//!
//! # The four transformations
//!
//! 1. **Hidden rustdoc lines are dropped.** A `#`-prefixed line inside a Rust fence is scaffolding
//!    rustdoc compiles and hides; a generic renderer would print it. The pass is fence-aware
//!    because `#` at column 0 outside a fence is a heading and `# ` inside a `python` fence is a
//!    real comment.
//! 2. **Marker triples become tab groups.** Consecutive markers sharing a name are one `<Tabs>`
//!    block. A `rust-only` marker becomes one tab under an admonition that names the symbols the
//!    marker already carries — the reason is not written twice.
//! 3. **Intra-doc links are resolved or dropped.** A sibling guide page becomes the site's route
//!    for it; a page of another crate's guide set becomes a link into the repository; an item path
//!    — a link to a `Deck` method, or to `std::error::Error::source` — becomes a plain code span,
//!    because nothing is published to docs.rs yet and a link to a page that does not exist is worse
//!    for a reader than none.
//! 4. **MDX hazards are escaped.** `{`, `}` and a `<` that opens what MDX would read as a tag are
//!    escaped outside code spans and fences, so prose that is legal CommonMark stays legal MDX.
//!
//! # What is deliberately not rendered
//!
//! The guide sets below the facade — `mjx-opc`, `mjx-dml`, `mjx-sml`, `mjx-chart` and the three
//! format crates. The bindings depend on `mjx-ooxml` alone, so a page about a crate underneath it
//! **cannot** be tri-language by the layering rule, and a user guide whose examples are Rust-only
//! in two thirds of its pages is a rustdoc mirror rather than a user guide. Those pages stay in
//! rustdoc, for contributors, and [`crate::docs_site`] links out to them where a facade page
//! already pointed a reader there.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

use crate::guide_examples::{self, Language, Marker, MARKER_END};

// ===============================================================================================
// Where things are
// ===============================================================================================

/// The site project, relative to the repository root.
pub const SITE_ROOT: &str = "site";

/// The generated content tree, relative to [`SITE_ROOT`].
///
/// Spelled in two halves rather than as one path so that no tracked document holds a code span
/// naming a directory that exists only after this command has run — `xtask/tests/doc_gate.rs`
/// resolves every such span against the working tree and would report it missing on a clean
/// checkout.
pub const DOCS_SUBDIRECTORY: &str = "docs";

/// The facade guide's module tree: the `include_str!` graph this generator derives its pages from.
pub const FACADE_GUIDE_MODULE: &str = "crates/mjx-ooxml/src/guide.rs";

/// The Python binding's guide module, which hosts the installation page for both bindings.
pub const PYTHON_GUIDE_MODULE: &str = "bindings/mjx-python/src/guide.rs";

/// The wasm binding's guide module, whose single page is about the npm package.
pub const WASM_GUIDE_MODULE: &str = "bindings/mjx-wasm/src/guide.rs";

/// Where the facade's walkthroughs live.
pub const WALKTHROUGH_DIRECTORY: &str = "crates/mjx-ooxml/examples";

/// The prefix a walkthrough's file name carries, which is also what separates one from a
/// `guide_<name>.rs` half.
pub const WALKTHROUGH_PREFIX: &str = "build_a_";

/// The two pages of the bindings' guide set a *user* needs before anything else works.
///
/// This is a list, and it is the only one in this module. The reason it is not derived is that the
/// other three pages of `mjx_python::guide` — the mapping rules, what is not projected, how much is
/// exercised — are about the *projection*, which is a contributor's question rather than a
/// reader's first one. Nothing in the repository distinguishes those two audiences, so nothing can
/// derive the split. Adding a page here is one line; the residue is recorded in MJXOFF-281.
const INSTALLATION_PAGES: [InstallationPage; 2] = [
    InstallationPage {
        module: PYTHON_GUIDE_MODULE,
        module_name: Some("installing"),
        slug: "installing",
        position: 1,
    },
    InstallationPage {
        module: WASM_GUIDE_MODULE,
        module_name: None,
        slug: "the_typescript_surface",
        position: 2,
    },
];

/// One entry of [`INSTALLATION_PAGES`]: which `include_str!` in which guide module.
struct InstallationPage {
    /// The guide module that includes the page, relative to the repository root.
    module: &'static str,
    /// The `pub mod` that holds it, or `None` when the page is the module's own root.
    module_name: Option<&'static str>,
    /// The page's file name on the site, without its extension.
    slug: &'static str,
    /// Its place in the section's sidebar.
    position: usize,
}

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

// ===============================================================================================
// The `include_str!` graph
// ===============================================================================================

/// One page a guide module includes: which markdown file, under which module, with which summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncludedPage {
    /// The markdown file, relative to the repository root.
    pub source: String,
    /// The `pub mod` that includes it, or `None` for the module's own root page.
    pub module: Option<String>,
    /// The one-line `///` summary above that module, if it has one.
    pub summary: Option<String>,
}

/// Every page a guide module pulls in with `include_str!`, in source order.
///
/// Source order **is** the reading order: `crates/mjx-ooxml/src/guide.rs` declares its modules in
/// the sequence its own index table lists them, and rustdoc renders them that way. Deriving the
/// sidebar from it means the two orders cannot disagree.
pub fn included_pages(root: &Path, module: &str) -> Result<Vec<IncludedPage>> {
    let text =
        std::fs::read_to_string(root.join(module)).with_context(|| format!("reading {module}"))?;
    let directory = Path::new(module)
        .parent()
        .ok_or_else(|| anyhow!("{module} has no parent directory"))?;

    let mut pages = Vec::new();
    let mut pending_summary: Option<String> = None;
    let mut pending_module: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("/// ") {
            pending_summary = Some(rest.trim().to_owned());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("pub mod ") {
            if let Some(name) = rest.strip_suffix(" {") {
                pending_module = Some(name.to_owned());
            }
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("#![doc = include_str!(\"") else {
            continue;
        };
        let Some(relative) = rest.split('"').next() else {
            bail!("{module}: an `include_str!` line with no closing quote");
        };
        let source = normalise(&directory.join(relative));
        if !root.join(&source).is_file() {
            bail!("{module}: `include_str!(\"{relative}\")` names {source}, which is not a file");
        }
        pages.push(IncludedPage {
            source,
            module: pending_module.take(),
            summary: pending_summary.take(),
        });
    }
    if pages.is_empty() {
        bail!(
            "{module}: no `include_str!` page was found, so the guide graph has stopped matching"
        );
    }
    Ok(pages)
}

/// A path with every `..` segment resolved, as a `/`-joined repository-relative string.
fn normalise(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component.as_os_str().to_string_lossy().as_ref() {
            "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other.to_owned()),
        }
    }
    parts.join("/")
}

/// The names a facade guide page's intra-doc links may resolve against.
///
/// Read out of `guide_vocabulary!` in the facade's guide module — the macro every page's module
/// invokes precisely so that its links resolve. A shortcut reference naming something outside it is
/// a link rustdoc would already refuse, and this generator refuses it too rather than rendering a
/// code span for a name that does not exist.
pub fn guide_vocabulary(root: &Path) -> Result<BTreeSet<String>> {
    let text = std::fs::read_to_string(root.join(FACADE_GUIDE_MODULE))
        .with_context(|| format!("reading {FACADE_GUIDE_MODULE}"))?;
    let opening = text
        .find("macro_rules! guide_vocabulary")
        .ok_or_else(|| anyhow!("{FACADE_GUIDE_MODULE}: no `guide_vocabulary!` macro"))?;
    let body = &text[opening..];
    let end = body
        .find("guide_vocabulary!();")
        .ok_or_else(|| anyhow!("{FACADE_GUIDE_MODULE}: `guide_vocabulary!` is never invoked"))?;
    // Every name is inside the braces of a `use …::{ … };`, which is the only shape the macro's
    // body has. Reading the braces rather than every identifier keeps `macro_rules`, `allow` and
    // the module path segments out of a set whose whole job is to say what a link may name.
    let mut names = BTreeSet::new();
    let mut rest = &body[..end];
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        for piece in rest[open + 1..open + close].split(',') {
            // A brace group can open mid-piece — `use crate::document::{BlockPath, RunPath}` —
            // so the name is what follows the last brace, and everything before it is the `use`.
            let name = piece.rsplit('{').next().unwrap_or(piece).trim();
            if !name.is_empty() && name.chars().all(is_name_character) {
                names.insert(name.to_owned());
            }
        }
        rest = &rest[open + close..];
    }
    if names.len() < 20 {
        bail!(
            "{FACADE_GUIDE_MODULE}: only {} vocabulary name(s) were read, so the macro parser has \
             stopped matching",
            names.len()
        );
    }
    Ok(names)
}

/// Whether a character can appear in a Rust identifier.
fn is_name_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

// ===============================================================================================
// The site's page model
// ===============================================================================================

/// One page of the generated site.
#[derive(Clone, Debug)]
pub struct SitePage {
    /// The markdown the page is rendered from, relative to the repository root.
    pub source: String,
    /// The section it sits in, or `None` for a page at the root of the docs tree.
    pub section: Option<String>,
    /// Its file name on the site, without an extension.
    pub slug: String,
    /// Its place in the section's sidebar.
    pub position: usize,
    /// The `///` summary of the module that includes it, when it has one.
    pub summary: Option<String>,
}

impl SitePage {
    /// The page's route, which is what another page links to it by.
    #[must_use]
    pub fn route(&self) -> String {
        match (&self.section, self.slug.as_str()) {
            (None, "intro") => "/".to_owned(),
            (None, slug) => format!("/{slug}"),
            (Some(section), slug) => format!("/{section}/{slug}"),
        }
    }

    /// Where the page is written, relative to the repository root.
    #[must_use]
    pub fn output(&self) -> String {
        let mut path = format!("{SITE_ROOT}/{DOCS_SUBDIRECTORY}");
        if let Some(section) = &self.section {
            path.push('/');
            path.push_str(section);
        }
        format!("{path}/{}.mdx", self.slug)
    }
}

/// One section of the sidebar, which Docusaurus reads out of a `_category_.json`.
struct Section {
    /// The directory under the docs tree.
    directory: &'static str,
    /// What the sidebar calls it.
    label: &'static str,
    /// Its place among the sections.
    position: usize,
}

/// The sections, in sidebar order. Their *contents* are derived; only the headings are named here.
const SECTIONS: [Section; 3] = [
    Section {
        directory: "install",
        label: "Installing",
        position: 2,
    },
    Section {
        directory: "guide",
        label: "The guide",
        position: 3,
    },
    Section {
        directory: "walkthroughs",
        label: "Walkthroughs",
        position: 4,
    },
];

/// Every page the site holds, derived from the `include_str!` graph and the examples directory.
///
/// **Both directions are the caller's to check** — see `xtask/tests/docs_site.rs`. What is
/// guaranteed here is only that nothing is listed: the facade pages come from the guide module, the
/// walkthroughs from the filesystem, and the two installation pages from the one table this module
/// admits to having.
pub fn site_pages(root: &Path) -> Result<Vec<SitePage>> {
    let mut pages = Vec::new();

    // ---- The facade guide -----------------------------------------------------------------------
    for (index, included) in included_pages(root, FACADE_GUIDE_MODULE)?
        .into_iter()
        .enumerate()
    {
        let page = match &included.module {
            None => SitePage {
                source: included.source,
                section: None,
                slug: "intro".to_owned(),
                position: 1,
                summary: included.summary,
            },
            Some(module) => SitePage {
                source: included.source,
                section: Some("guide".to_owned()),
                slug: module.clone(),
                position: index,
                summary: Some(included.summary.clone().ok_or_else(|| {
                    anyhow!(
                        "{FACADE_GUIDE_MODULE}: `pub mod {module}` has no `///` summary, and the \
                         site's description of the page is that summary"
                    )
                })?),
            },
        };
        pages.push(page);
    }

    // ---- Installing -----------------------------------------------------------------------------
    for entry in &INSTALLATION_PAGES {
        let included = included_pages(root, entry.module)?;
        let found = included
            .iter()
            .find(|page| page.module.as_deref() == entry.module_name)
            .ok_or_else(|| {
                anyhow!(
                    "{}: no `include_str!` under {:?}, so the installation page has moved",
                    entry.module,
                    entry.module_name.unwrap_or("the module root")
                )
            })?;
        pages.push(SitePage {
            source: found.source.clone(),
            section: Some("install".to_owned()),
            slug: entry.slug.to_owned(),
            position: entry.position,
            summary: found.summary.clone(),
        });
    }

    // ---- The walkthroughs -------------------------------------------------------------------------
    for (index, walkthrough) in walkthroughs(root)?.into_iter().enumerate() {
        pages.push(SitePage {
            source: walkthrough.rust.clone(),
            section: Some("walkthroughs".to_owned()),
            slug: walkthrough.name.clone(),
            position: index + 1,
            summary: None,
        });
    }

    Ok(pages)
}

// ===============================================================================================
// The walkthroughs
// ===============================================================================================

/// One walkthrough, and the three files that are the same program in three languages.
#[derive(Clone, Debug)]
pub struct Walkthrough {
    /// `build_a_deck`, which is also the site's file name for it.
    pub name: String,
    /// The `cargo` example, relative to the repository root.
    pub rust: String,
    /// The `pytest` module that is the same program through the Python binding.
    pub python: String,
    /// The `node --test` module that is the same program through the wasm binding.
    pub javascript: String,
}

/// Every walkthrough, derived from the examples directory rather than listed.
///
/// The same population `xtask/tests/walkthrough_triples.rs` derives, for the same reason: a fourth
/// walkthrough must not be able to arrive with no page, the way the Word one arrived with no
/// comparison. A walkthrough missing either twin is an error here rather than a page with two tabs.
pub fn walkthroughs(root: &Path) -> Result<Vec<Walkthrough>> {
    let directory = root.join(WALKTHROUGH_DIRECTORY);
    let entries = std::fs::read_dir(&directory)
        .with_context(|| format!("reading {}", directory.display()))?;
    let mut names: BTreeSet<String> = BTreeSet::new();
    for entry in entries {
        let entry = entry.with_context(|| format!("reading {}", directory.display()))?;
        let file = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = file.strip_suffix(".rs") else {
            continue;
        };
        if stem.starts_with(WALKTHROUGH_PREFIX) {
            names.insert(stem.to_owned());
        }
    }
    if names.is_empty() {
        bail!(
            "{WALKTHROUGH_DIRECTORY}: no `{WALKTHROUGH_PREFIX}*.rs` was found, so the walkthrough \
             walk has stopped matching"
        );
    }

    let mut found = Vec::new();
    for name in names {
        let rust = format!("{WALKTHROUGH_DIRECTORY}/{name}.rs");
        let python = format!("bindings/mjx-python/tests/test_{name}.py");
        let javascript = format!("bindings/mjx-wasm/tests/node/{name}.mjs");
        for twin in [&python, &javascript] {
            if !root.join(twin).is_file() {
                bail!(
                    "{rust} is a walkthrough and {twin} is not there. Every walkthrough exists in \
                     all three languages — `xtask/tests/walkthrough_triples.rs` is what holds the \
                     comparison to that, and a page with two tabs would quietly say otherwise."
                );
            }
        }
        found.push(Walkthrough {
            name,
            rust,
            python,
            javascript,
        });
    }
    Ok(found)
}

// ===============================================================================================
// Reading a page: prose, fences, and marked blocks
// ===============================================================================================

/// One contiguous run of a markdown page.
#[derive(Clone, Debug)]
enum Chunk {
    /// Everything that is not a fence: the text the link and escape passes rewrite.
    Prose(Vec<String>),
    /// A fenced block the page wrote by hand.
    Fence { tag: String, body: Vec<String> },
    /// A fenced block a marker owns, which becomes one tab.
    Example { marker: Marker, body: Vec<String> },
}

/// A page, split into the runs the renderer treats differently.
///
/// The marker scan is [`guide_examples::parse_marker_line`] itself rather than a second reader of
/// the same syntax, so a marker this generator does not understand is one the copier does not
/// understand either.
fn chunks(page: &str, text: &str) -> Result<Vec<Chunk>> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<Chunk> = Vec::new();
    let mut prose: Vec<String> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];

        if let Some(marker) = guide_examples::parse_marker_line(line, page, index + 1)? {
            if !prose.is_empty() {
                out.push(Chunk::Prose(std::mem::take(&mut prose)));
            }
            let opener = lines
                .get(index + 1)
                .ok_or_else(|| anyhow!("{page}:{}: a marked block ends the file", marker.line))?;
            let Some(tag) = opener.strip_prefix("```") else {
                bail!(
                    "{page}:{}: a marked block opens a fence on the next line, and this one is \
                     {opener:?}",
                    marker.line
                );
            };
            if tag.trim() != marker.language.token() {
                bail!(
                    "{page}:{}: the marker says `{}` and the fence says {:?}. The copier writes \
                     both, so the two disagreeing means one of them was edited by hand.",
                    marker.line,
                    marker.language.token(),
                    tag.trim()
                );
            }
            let mut body = Vec::new();
            let mut cursor = index + 2;
            while cursor < lines.len() && lines[cursor].trim_end() != "```" {
                body.push(lines[cursor].to_owned());
                cursor += 1;
            }
            if cursor >= lines.len() {
                bail!(
                    "{page}:{}: this marked block's fence is never closed",
                    marker.line
                );
            }
            let closer = lines.get(cursor + 1).map(|line| line.trim()).unwrap_or("");
            if closer != MARKER_END {
                bail!(
                    "{page}:{}: a marked block is closed by `{MARKER_END}`, and this one is \
                     followed by {closer:?}",
                    marker.line
                );
            }
            out.push(Chunk::Example { marker, body });
            index = cursor + 2;
            continue;
        }

        if let Some(tag) = line.strip_prefix("```") {
            if !prose.is_empty() {
                out.push(Chunk::Prose(std::mem::take(&mut prose)));
            }
            let mut body = Vec::new();
            let mut cursor = index + 1;
            while cursor < lines.len() && !lines[cursor].starts_with("```") {
                body.push(lines[cursor].to_owned());
                cursor += 1;
            }
            if cursor >= lines.len() {
                bail!("{page}:{}: this fence is never closed", index + 1);
            }
            out.push(Chunk::Fence {
                tag: tag.trim().to_owned(),
                body,
            });
            index = cursor + 1;
            continue;
        }

        prose.push(line.to_owned());
        index += 1;
    }
    if !prose.is_empty() {
        out.push(Chunk::Prose(prose));
    }
    Ok(out)
}

/// Whether a fence tag names Rust, so its `#` lines are rustdoc's hidden ones.
///
/// An untagged fence counts, because that is rustdoc's own rule. A `python` fence does **not**,
/// which is the whole reason this decision is made per fence rather than per line: `# a comment` is
/// scaffolding in one language and content in the next.
fn is_rust_fence(tag: &str) -> bool {
    let tag = tag.trim();
    if tag.is_empty() {
        return true;
    }
    tag.split(',').any(|word| {
        matches!(
            word.trim(),
            "rust" | "no_run" | "ignore" | "should_panic" | "compile_fail"
        )
    })
}

/// A Rust block with rustdoc's hidden lines removed and its `##` escapes unescaped.
///
/// rustdoc hides a line whose first non-blank character is `#` followed by a space or nothing, and
/// treats a leading `##` as an escaped `#`. Both rules are applied here so that a block a reader
/// sees on the site is the block a reader sees on docs.rs.
fn without_hidden_lines(body: &[String]) -> Vec<String> {
    let mut kept = Vec::with_capacity(body.len());
    for line in body {
        let trimmed = line.trim_start();
        if trimmed == "#" || trimmed.starts_with("# ") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("##") {
            let indent = &line[..line.len() - trimmed.len()];
            kept.push(format!("{indent}#{rest}"));
            continue;
        }
        kept.push(line.clone());
    }
    kept
}

// ===============================================================================================
// Links
// ===============================================================================================

/// What one intra-doc link target becomes on the site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// A URL a reader can follow.
    Url(String),
    /// No target: the link text is emitted on its own.
    ///
    /// This is what an item path becomes. Nothing in this workspace is published to docs.rs, so a
    /// link to a `Deck` method has no page to point at, and a link to a page that does not exist is
    /// worse for a reader than a code span that does not pretend to be one.
    Plain,
}

/// Everything the link pass needs that is not the link itself.
struct LinkContext<'a> {
    /// The page being rendered, for a failure message.
    page: &'a str,
    /// Every site page, so a sibling link becomes a route.
    routes: &'a BTreeMap<String, String>,
    /// Where a link into the repository points.
    blob_base: &'a str,
    /// Whether a path exists in the working tree.
    root: &'a Path,
    /// The names a shortcut reference may name, or empty on a page that has no vocabulary.
    vocabulary: &'a BTreeSet<String>,
}

impl LinkContext<'_> {
    /// A link into the repository, when the path it names is really there.
    fn blob(&self, path: &str) -> Option<String> {
        self.root
            .join(path)
            .exists()
            .then(|| format!("{}/{path}", self.blob_base))
    }
}

/// What one inline link's target becomes.
///
/// The classification is the one `xtask/tests/doc_gate.rs` already uses in the other direction: a
/// target with a `::` is a rustdoc item path and never a file. Everything else is a page.
fn resolve_target(target: &str, context: &LinkContext<'_>) -> Result<Resolution> {
    if target.starts_with('#')
        || target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
    {
        return Ok(Resolution::Url(target.to_owned()));
    }

    let (path, anchor) = match target.split_once('#') {
        Some((path, anchor)) => (path, format!("#{anchor}")),
        None => (target, String::new()),
    };

    if path.contains("::") {
        // The facade's own guide set, which is what the site renders: a page of it resolves to
        // that page's route.
        if let Some(rest) = path.strip_prefix("crate::guide") {
            let slug = rest.trim_start_matches("::");
            let key = if slug.is_empty() { "intro" } else { slug };
            if let Some(route) = context.routes.get(key) {
                return Ok(Resolution::Url(format!("{route}{anchor}")));
            }
        }
        // A documentation-only module of the facade whose page is not on the site but is in the
        // repository — the shared-markup reachability table is the one instance.
        if let Some(module) = path.strip_prefix("crate::") {
            if !module.contains("::") {
                if let Some(url) = context.blob(&format!("crates/mjx-ooxml/docs/{module}.md")) {
                    return Ok(Resolution::Url(format!("{url}{anchor}")));
                }
            }
        }
        // `mjx_pptx::guide`, `mjx_xlsx::guide::deliberate_limitations` — another crate's guide set,
        // deliberately off the site. It is a real page in the repository, so it gets a real link.
        if let Some((krate, rest)) = path.split_once("::guide") {
            if let Some(name) = krate.strip_prefix("mjx_") {
                let directory = format!("crates/mjx-{}/docs/guide", name.replace('_', "-"));
                let leaf = rest.trim_start_matches("::");
                let file = if leaf.is_empty() { "README" } else { leaf };
                if let Some(url) = context.blob(&format!("{directory}/{file}.md")) {
                    return Ok(Resolution::Url(format!("{url}{anchor}")));
                }
            }
        }
        return Ok(Resolution::Plain);
    }

    if path.contains('/') {
        return Ok(context.blob(path).map_or(Resolution::Plain, |url| {
            Resolution::Url(format!("{url}{anchor}"))
        }));
    }

    // A bare word is a sibling page of the same guide set, which is how every one of these guides
    // links to its neighbours.
    if let Some(route) = context.routes.get(path) {
        return Ok(Resolution::Url(format!("{route}{anchor}")));
    }
    bail!(
        "{}: `]({target})` names neither a site page, a repository path nor an item. A sibling \
         link whose page is not on the site would render as a dead link, so it is refused here \
         rather than shipped.",
        context.page
    )
}

/// Whether a shortcut reference — `` [`Deck::save`] `` — names something the page may link to.
fn shortcut_is_in_vocabulary(name: &str, vocabulary: &BTreeSet<String>) -> bool {
    if vocabulary.is_empty() {
        return true;
    }
    let base = name
        .trim_start_matches('&')
        .split("::")
        .next()
        .unwrap_or(name)
        .trim_end_matches("()");
    matches!(base, "std" | "core" | "alloc")
        || base.starts_with("mjx_")
        || vocabulary.contains(base)
}

// ===============================================================================================
// Prose: links, then MDX escaping
// ===============================================================================================

/// One run of prose, with its links resolved and its MDX hazards escaped.
///
/// A single left-to-right scan, which is what keeps a code span out of every other pass: the scan
/// copies a span verbatim the moment it meets its opening backtick, so nothing inside one is ever
/// read as a link or as a brace. Masking the text and running patterns over the mask would be the
/// same idea with two representations to keep in step.
fn render_prose(text: &str, context: &LinkContext<'_>) -> Result<String> {
    let characters: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < characters.len() {
        match characters[index] {
            '`' => {
                let end = code_span_end(&characters, index);
                out.extend(&characters[index..end]);
                index = end;
            }
            '[' => match link_at(&characters, index) {
                Some(link) => {
                    out.push_str(&render_link(&characters, &link, context)?);
                    index = link.end;
                }
                None => {
                    out.push('[');
                    index += 1;
                }
            },
            '{' => {
                out.push_str("&#123;");
                index += 1;
            }
            '}' => {
                out.push_str("&#125;");
                index += 1;
            }
            '<' if opens_a_tag(&characters, index) => {
                out.push_str("&lt;");
                index += 1;
            }
            character => {
                out.push(character);
                index += 1;
            }
        }
    }
    Ok(out)
}

/// Where the code span opening at `start` ends, one past its closing backtick run.
///
/// A run of *n* backticks is closed by the next run of exactly *n*. An unclosed run is not a code
/// span at all, and the scan then treats its opening backtick as an ordinary character.
fn code_span_end(characters: &[char], start: usize) -> usize {
    let mut width = 0;
    while start + width < characters.len() && characters[start + width] == '`' {
        width += 1;
    }
    let mut index = start + width;
    while index < characters.len() {
        if characters[index] != '`' {
            index += 1;
            continue;
        }
        let mut run = 0;
        while index + run < characters.len() && characters[index + run] == '`' {
            run += 1;
        }
        if run == width {
            return index + run;
        }
        index += run;
    }
    start + 1
}

/// Whether the `<` at `index` opens what MDX would read as a tag.
fn opens_a_tag(characters: &[char], index: usize) -> bool {
    matches!(
        characters.get(index + 1),
        Some(next) if next.is_ascii_alphabetic() || *next == '/' || *next == '!'
    )
}

/// One markdown link, found by the scan.
struct Link {
    /// Where the link text starts, one past the `[`.
    text: (usize, usize),
    /// The inline target, when the link has one. A shortcut reference has none.
    target: Option<(usize, usize)>,
    /// One past the link's last character.
    end: usize,
}

/// The link opening at `start`, if one opens there.
///
/// Code spans are skipped while the closing `]` is looked for, so `` [`Deck::save`] `` is one link
/// whose text happens to be a code span rather than three tokens.
fn link_at(characters: &[char], start: usize) -> Option<Link> {
    let mut index = start + 1;
    while index < characters.len() {
        match characters[index] {
            '`' => index = code_span_end(characters, index),
            ']' => break,
            '\n' if characters.get(index + 1) == Some(&'\n') => return None,
            _ => index += 1,
        }
    }
    if index >= characters.len() || characters[index] != ']' {
        return None;
    }
    let text = (start + 1, index);
    match characters.get(index + 1) {
        Some('(') => {
            let mut cursor = index + 2;
            while cursor < characters.len() && characters[cursor] != ')' {
                cursor += 1;
            }
            if cursor >= characters.len() {
                return None;
            }
            Some(Link {
                text,
                target: Some((index + 2, cursor)),
                end: cursor + 1,
            })
        }
        // A reference link or a link definition: neither occurs in this corpus, and either would
        // be carried through unchanged rather than half-rewritten.
        Some('[') | Some(':') => None,
        _ => Some(Link {
            text,
            target: None,
            end: index + 1,
        }),
    }
}

/// One link, rendered.
fn render_link(characters: &[char], link: &Link, context: &LinkContext<'_>) -> Result<String> {
    let text: String = characters[link.text.0..link.text.1].iter().collect();
    let Some((from, to)) = link.target else {
        // A shortcut reference. Its text is already a code span in every instance this corpus has,
        // and a code span is exactly what it should become.
        let name = text.trim().trim_matches('`');
        if !shortcut_is_in_vocabulary(name, context.vocabulary) {
            bail!(
                "{}: `[{text}]` is a shortcut reference to {name:?}, which is not in \
                 `guide_vocabulary!`. rustdoc would refuse the link; rendering it as a code span \
                 here would hide a name that does not exist.",
                context.page
            );
        }
        return Ok(text);
    };
    let target: String = characters[from..to].iter().collect();
    let rendered_text = render_prose(&text, context)?;
    match resolve_target(target.trim(), context)? {
        Resolution::Url(url) => Ok(format!("[{rendered_text}]({url})")),
        Resolution::Plain => Ok(rendered_text),
    }
}

// ===============================================================================================
// Rendering a page
// ===============================================================================================

/// What one language's tab is labelled.
///
/// `TypeScript` over a `js` fence is deliberate and is the one place this generator calls something
/// by a different name than the repository does. The file `node --test` runs is JavaScript — see
/// [`Language::token`] for why the marker says `js` — but what the npm package *ships* is a typed
/// surface, and `TypeScript` is what the reader who came looking for it is looking for. The fence
/// stays `js`, so the highlighting and the extension still say what the file is.
fn tab_label(language: Language) -> &'static str {
    match language {
        Language::Rust => "Rust",
        Language::Python => "Python",
        Language::JavaScript => "TypeScript",
    }
}

/// One tab group: an example's halves, in the order a marked section presents them.
fn render_tabs(group: &[(&Marker, &[String])]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some((marker, _)) = group.first() {
        if marker.is_rust_only() {
            let names = marker
                .rust_only
                .iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push(":::note[Rust only]".to_owned());
            out.push(format!(
                "This block has no Python or TypeScript half, by decision: {names} {} reachable \
                 from neither binding. That is a claim the repository checks on every run — the \
                 day a binding projects one of those names, the guide's own gate fails.",
                if marker.rust_only.len() == 1 {
                    "is"
                } else {
                    "are"
                }
            ));
            out.push(":::".to_owned());
            out.push(String::new());
        }
    }
    out.push("<Tabs groupId=\"language\">".to_owned());
    for (marker, body) in group {
        let language = marker.language;
        out.push(format!(
            "<TabItem value=\"{}\" label=\"{}\">",
            language.token(),
            tab_label(language)
        ));
        out.push(String::new());
        out.push(format!("```{}", language.token()));
        out.extend(if is_rust_fence(language.token()) {
            without_hidden_lines(body)
        } else {
            body.to_vec()
        });
        out.push("```".to_owned());
        out.push(String::new());
        out.push("</TabItem>".to_owned());
    }
    out.push("</Tabs>".to_owned());
    out
}

/// A markdown body, rendered to MDX.
///
/// Returns the rendered lines and whether any tab group was emitted, because the `<Tabs>` import is
/// only written on a page that has one.
fn render_body(page: &str, text: &str, context: &LinkContext<'_>) -> Result<(Vec<String>, usize)> {
    let parsed = chunks(page, text)?;
    let mut out: Vec<String> = Vec::new();
    let mut groups = 0usize;
    let mut index = 0;
    while index < parsed.len() {
        match &parsed[index] {
            Chunk::Prose(lines) => {
                let rendered = render_prose(&lines.join("\n"), context)?;
                // `split` rather than `lines`, so a prose run that ends in a blank line keeps it.
                // `"a\n".lines()` is one element and `"a\n".split('\n')` is two, and the second
                // one is the blank line that separates a paragraph from the fence below it.
                out.extend(rendered.split('\n').map(str::to_owned));
                index += 1;
            }
            Chunk::Fence { tag, body } => {
                out.push(format!("```{tag}"));
                out.extend(if is_rust_fence(tag) {
                    without_hidden_lines(body)
                } else {
                    body.clone()
                });
                out.push("```".to_owned());
                index += 1;
            }
            Chunk::Example { marker, .. } => {
                // The three halves of one example are three marked blocks separated by the blank
                // line between them, so the blank run is walked over rather than treated as the end
                // of the group — but only when another half of the *same* example follows it.
                // Otherwise `saving_validates` and `the_round_trip` would become one tab group with
                // six tabs.
                let name = marker.name.clone();
                let mut group: Vec<(&Marker, &[String])> = Vec::new();
                loop {
                    match parsed.get(index) {
                        Some(Chunk::Example { marker, body }) if marker.name == name => {
                            group.push((marker, body.as_slice()));
                            index += 1;
                        }
                        Some(Chunk::Prose(lines))
                            if lines.iter().all(|line| line.trim().is_empty())
                                && matches!(
                                    parsed.get(index + 1),
                                    Some(Chunk::Example { marker, .. }) if marker.name == name
                                ) =>
                        {
                            index += 1;
                        }
                        _ => break,
                    }
                }
                out.push(String::new());
                out.extend(render_tabs(&group));
                out.push(String::new());
                groups += 1;
            }
        }
    }
    Ok((out, groups))
}

/// A value, quoted for YAML front matter.
fn quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// The page's title and the body with its own `#` heading removed.
///
/// Docusaurus renders the front matter's title as the page's `h1`, so leaving the markdown one in
/// place would print it twice.
fn split_title(page: &str, lines: &[String]) -> Result<(String, Vec<String>)> {
    let position = lines
        .iter()
        .position(|line| line.starts_with("# "))
        .ok_or_else(|| anyhow!("{page}: no `# ` heading, so the page has no title"))?;
    let title = lines[position][2..].trim().to_owned();
    let mut body: Vec<String> = Vec::new();
    body.extend_from_slice(&lines[..position]);
    body.extend_from_slice(&lines[position + 1..]);
    while body.first().is_some_and(|line| line.trim().is_empty()) {
        body.remove(0);
    }
    Ok((title, body))
}

/// One rendered page, ready to write.
#[derive(Clone, Debug)]
pub struct RenderedPage {
    /// The markdown or Rust source it was rendered from, relative to the repository root.
    pub source: String,
    /// Where it is written, relative to the repository root.
    pub path: String,
    /// Its full MDX text.
    pub text: String,
    /// How many tab groups it holds.
    pub tab_groups: usize,
    /// How many tabs those groups hold between them.
    pub tabs: usize,
}

/// Every page of the site, rendered.
pub fn render(root: &Path) -> Result<Vec<RenderedPage>> {
    let pages = site_pages(root)?;
    let vocabulary = guide_vocabulary(root)?;
    let blob_base = blob_base(root)?;
    let routes: BTreeMap<String, String> = pages
        .iter()
        .map(|page| (page.slug.clone(), page.route()))
        .collect();

    let mut rendered = Vec::new();
    for page in &pages {
        // Only the facade's own guide set has a vocabulary; the bindings' pages deliberately carry
        // no intra-doc item links, and the walkthroughs are Rust source rather than a guide page.
        let scoped: BTreeSet<String> = if page.source.starts_with("crates/mjx-ooxml/docs/guide/") {
            vocabulary.clone()
        } else {
            BTreeSet::new()
        };
        let context = LinkContext {
            page: &page.source,
            routes: &routes,
            blob_base: &blob_base,
            root,
            vocabulary: &scoped,
        };

        let (title, body, tab_groups, tabs) = if page.section.as_deref() == Some("walkthroughs") {
            render_walkthrough(root, page, &context)?
        } else {
            let text = std::fs::read_to_string(root.join(&page.source))
                .with_context(|| format!("reading {}", page.source))?;
            let (lines, groups) = render_body(&page.source, &text, &context)?;
            let tabs = count_tabs(&lines);
            let (title, body) = split_title(&page.source, &lines)?;
            (title, body, groups, tabs)
        };

        let mut text = String::new();
        text.push_str("---\n");
        let _ = writeln!(text, "id: {}", page.slug);
        let _ = writeln!(text, "title: {}", quoted(&title));
        if let Some(summary) = &page.summary {
            let _ = writeln!(text, "description: {}", quoted(summary));
        }
        let _ = writeln!(text, "sidebar_position: {}", page.position);
        if page.route() == "/" {
            text.push_str("slug: /\n");
        }
        text.push_str("---\n\n");
        if tab_groups > 0 {
            text.push_str("import Tabs from '@theme/Tabs';\n");
            text.push_str("import TabItem from '@theme/TabItem';\n\n");
        }
        text.push_str(&body.join("\n"));
        if !text.ends_with('\n') {
            text.push('\n');
        }

        rendered.push(RenderedPage {
            source: page.source.clone(),
            path: page.output(),
            text,
            tab_groups,
            tabs,
        });
    }
    Ok(rendered)
}

/// How many `<TabItem>` openers a rendered body holds.
fn count_tabs(lines: &[String]) -> usize {
    lines
        .iter()
        .filter(|line| line.starts_with("<TabItem "))
        .count()
}

/// A walkthrough's page: the example's own header prose, then all three programs as tabs.
///
/// The three files are emitted **whole**, because a walkthrough is a whole program and the point of
/// showing it in three languages is that a reader can see one of them end to end. They are the
/// files `cargo`, `pytest` and `node --test` run, so what the page shows and what CI executes
/// cannot diverge.
fn render_walkthrough(
    root: &Path,
    page: &SitePage,
    context: &LinkContext<'_>,
) -> Result<(String, Vec<String>, usize, usize)> {
    let walkthrough = walkthroughs(root)?
        .into_iter()
        .find(|candidate| candidate.name == page.slug)
        .ok_or_else(|| anyhow!("{}: no walkthrough of that name", page.slug))?;

    let source = std::fs::read_to_string(root.join(&walkthrough.rust))
        .with_context(|| format!("reading {}", walkthrough.rust))?;
    let header: String = source
        .lines()
        .take_while(|line| line.starts_with("//!"))
        .map(|line| line.strip_prefix("//! ").unwrap_or("").to_owned())
        .collect::<Vec<_>>()
        .join("\n");
    if header.trim().is_empty() {
        bail!(
            "{}: no `//!` header, and the page's prose is that header",
            walkthrough.rust
        );
    }
    let (prose, _) = render_body(&walkthrough.rust, &header, context)?;

    let title = page
        .slug
        .replace('_', " ")
        .split_whitespace()
        .enumerate()
        .map(|(index, word)| {
            if index == 0 {
                let mut characters = word.chars();
                match characters.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                    None => String::new(),
                }
            } else {
                word.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut body = prose;
    body.push(String::new());
    body.push("## The program, in all three languages".to_owned());
    body.push(String::new());
    body.push(
        "Each tab is a whole file a test runner executes, verbatim. The Python and TypeScript ones \
         are also the modules that compare their own package against the Rust one part by part, \
         byte for byte, which is why they carry a little test scaffolding the Rust example does \
         not."
            .to_owned(),
    );
    body.push(String::new());
    body.push("<Tabs groupId=\"language\">".to_owned());
    for (language, path) in [
        (Language::Rust, &walkthrough.rust),
        (Language::Python, &walkthrough.python),
        (Language::JavaScript, &walkthrough.javascript),
    ] {
        let text =
            std::fs::read_to_string(root.join(path)).with_context(|| format!("reading {path}"))?;
        body.push(format!(
            "<TabItem value=\"{}\" label=\"{}\">",
            language.token(),
            tab_label(language)
        ));
        body.push(String::new());
        body.push(format!("```{} title=\"{path}\"", language.token()));
        body.extend(text.lines().map(str::to_owned));
        body.push("```".to_owned());
        body.push(String::new());
        body.push("</TabItem>".to_owned());
    }
    body.push("</Tabs>".to_owned());

    Ok((title, body, 1, 3))
}

/// Where a link into the repository points, read from the workspace manifest.
fn blob_base(root: &Path) -> Result<String> {
    let manifest =
        std::fs::read_to_string(root.join("Cargo.toml")).context("reading Cargo.toml")?;
    let line = manifest
        .lines()
        .find(|line| line.trim_start().starts_with("repository = "))
        .ok_or_else(|| anyhow!("Cargo.toml: no `repository` key to link into"))?;
    let url = line
        .split('"')
        .nth(1)
        .ok_or_else(|| anyhow!("Cargo.toml: the `repository` key is not a quoted string"))?;
    Ok(format!("{}/blob/main", url.trim_end_matches('/')))
}

// ===============================================================================================
// The command
// ===============================================================================================

/// Write the site's content tree, and say what it holds.
pub fn run() -> Result<()> {
    let root = repository_root();
    let pages = render(&root)?;

    let docs = root.join(SITE_ROOT).join(DOCS_SUBDIRECTORY);
    if docs.exists() {
        std::fs::remove_dir_all(&docs).with_context(|| format!("clearing {}", docs.display()))?;
    }
    for section in &SECTIONS {
        let directory = docs.join(section.directory);
        std::fs::create_dir_all(&directory)
            .with_context(|| format!("creating {}", directory.display()))?;
        let category = format!(
            "{{\n  \"label\": \"{}\",\n  \"position\": {}\n}}\n",
            section.label, section.position
        );
        std::fs::write(directory.join("_category_.json"), category)
            .with_context(|| format!("writing {}", directory.display()))?;
    }

    let mut tab_groups = 0usize;
    let mut tabs = 0usize;
    for page in &pages {
        let path = root.join(&page.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&path, &page.text).with_context(|| format!("writing {}", page.path))?;
        tab_groups += page.tab_groups;
        tabs += page.tabs;
    }
    println!(
        "{} page(s), {tab_groups} tab group(s), {tabs} tab(s) written under {SITE_ROOT}/{DOCS_SUBDIRECTORY}",
        pages.len()
    );
    Ok(())
}
