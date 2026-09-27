//! `xtask validation-artefacts` — the artefacts the human Office pass reads (MJXOFF-122).
//!
//! `--ingest <file>` runs the same command the other way: hand it something saved out of Office and
//! it reports which entry the file answers, whether it round-trips, whether the package holds,
//! whether its child order matches ours, whether it validates, and where it would be committed
//! (MJXOFF-130). See [`ingest`] for the line between a finding that is this library's and one that
//! is the file's.
//!
//! # What this command is for
//!
//! Every fixture in this repository was written by this project or by LibreOffice, and no test has
//! ever read a file Microsoft Office wrote. The schema gate, the package validator and the
//! child-order audit answer *"is this markup well-formed and in the order the schema says"*; none of
//! them can answer *"does real Office render what we intended"*. That question needs a person with
//! Office in front of them, and this command produces the files that person opens.
//!
//! # What it does not do
//!
//! **It marks nothing.** There is no verdict in this module, no `pass`, and no inference from a
//! LibreOffice conversion. The result columns in `docs/validation/` are the user's to fill; see
//! `docs/validation/00-method.md` for the convention and for the rule that keeps the exercise
//! honest — *a documented gap is never a validation failure*.
//!
//! # Two variants per area, and why both
//!
//! * **Authored** — built from [`Deck::blank`](mjx_ooxml::Deck::blank),
//!   [`Document::blank`](mjx_ooxml::Document::blank) or
//!   [`Workbook::blank`](mjx_ooxml::Workbook::blank). Nothing is read from disk, so every byte in
//!   the artefact is one this library wrote.
//! * **Edited** — produced by *editing* an Office-authored original from
//!   [`corpus::CORPUS_DIRECTORY`]. Authoring bugs and editing bugs are different bugs, and only this
//!   variant exercises tier-3 edit isolation against markup we did not write.
//!
//! The corpus is **empty**, and no agent may fill it — the value of an Office-authored file is
//! entirely its provenance — so every edit variant **skips by name**: it prints the area it skipped
//! and the path it looked for, never a silent absence. `MJX_REQUIRE_OFFICE_CORPUS=1` turns any such
//! skip into a hard failure, exactly as `MJX_REQUIRE_SOFFICE=1` does for the `office_open` canary.
//! `docs/validation/06-the-office-pass.md` §5 is how a person fills it, and [`ingest`] is the
//! command that reports on a file before it goes in.
//!
//! # Everything goes through the facade
//!
//! Not one generator names a crate below `mjx-ooxml`. That is deliberate: the human pass then
//! validates the facade and its error mapping as a side effect, and the same calls are what the two
//! bindings make, which is what lets `bindings/mjx-python/tests/test_validation_artefacts.py` and
//! `bindings/mjx-wasm/tests/node/validation_artefacts.mjs` compare their artefacts against these
//! ones part by part, byte for byte.
//!
//! # Determinism
//!
//! Two runs produce byte-identical artefacts. Without that the three-language comparison means
//! nothing and every future diff is noise, so `xtask/tests/validation_harness.rs` asserts it by
//! generating twice into two directories and comparing. Note where the file I/O is: in this module.
//! The library is bytes in, bytes out, and an artefact generator is not a licence to change that.

mod corpus;
mod document;
mod ingest;
mod model;
mod presentation;
mod workbook;

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

pub use corpus::{
    corpus_directory, corpus_files, original_for, original_path, CorpusFile, CORPUS_DIRECTORY,
};
pub use ingest::{
    area_for_file_name, area_for_token, format_for_file_name, ingest, model_findings, report,
    Finding, IngestReport, Verdict,
};

/// Which of the three formats an area belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtefactFormat {
    /// PresentationML — `Deck`.
    Presentation,
    /// WordprocessingML — `Document`.
    Document,
    /// SpreadsheetML — `Workbook`.
    Workbook,
}

impl ArtefactFormat {
    /// The conventional extension, which is also the `--format` token.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Presentation => "pptx",
            Self::Document => "docx",
            Self::Workbook => "xlsx",
        }
    }

    /// The three, in the order the report prints them.
    #[must_use]
    pub fn all() -> [Self; 3] {
        [Self::Presentation, Self::Document, Self::Workbook]
    }

    /// The catalogue page this format's areas are written on, as a `docs/validation/` file name.
    ///
    /// The pages of `docs/validation/` are not interchangeable — some are method, index, risk order
    /// and the Office hand-off, and the rest are one per format — so a test that wants *the format
    /// pages* needs to say which, and the honest place for that is here, beside [`Self::extension`],
    /// where a new format would have to answer the same question. Before MJXOFF-225 they were
    /// spelled out as a literal in `xtask/tests/validation_calls.rs`, which is the shape that makes
    /// a sweep silently partial.
    #[must_use]
    pub fn page(self) -> &'static str {
        match self {
            Self::Presentation => "03-presentations.md",
            Self::Document => "04-documents.md",
            Self::Workbook => "05-workbooks.md",
        }
    }

    /// The format a `--format` token names, or `None`.
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        Self::all().into_iter().find(|f| f.extension() == token)
    }
}

/// How much of what an area covers is a *judgement* Office could disagree with, rather than markup
/// a schema already pins down.
///
/// This is the risk level `docs/validation/00-method.md` defines, and it is stated here as well as
/// there **on purpose**: `xtask/tests/validation_index.rs` compares the two, so a level changed in
/// one place and not the other fails rather than drifting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Risk {
    /// Inheritance, resolution or style layering — where a reading of the prose, not a schema, is
    /// what decides the rendered answer. LibreOffice diverges from Office most here.
    High,
    /// Modelled markup whose shape the schema pins down but whose *rendering* still has choices in
    /// it — geometry, effects, anchoring.
    Medium,
    /// Markup with essentially one rendering, present so the pass covers the area at all.
    Low,
}

impl Risk {
    /// The word the documents use.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    /// The level a word names, or `None`. The index test uses it to hold the documents' spelling
    /// of a level to the set this enumeration has.
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        [Self::High, Self::Medium, Self::Low]
            .into_iter()
            .find(|risk| risk.label() == token)
    }
}

/// Which of an area's two artefacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// Built from `blank`, with nothing read from disk.
    Authored,
    /// Built by editing an Office-authored original.
    Edited,
}

impl Variant {
    /// The token that appears in an artefact's file name.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Authored => "authored",
            Self::Edited => "edited",
        }
    }

    /// Both, in the order the report prints them.
    #[must_use]
    pub fn all() -> [Self; 2] {
        [Self::Authored, Self::Edited]
    }
}

/// One validation area: a coherent rendering-risk theme, one entry in `docs/validation/01-index.md`,
/// and two artefacts.
#[derive(Debug)]
pub struct Area {
    /// The entry id, e.g. `V-PPTX-01`. `docs/validation/00-method.md` states the scheme.
    pub id: &'static str,
    /// Which format this area belongs to.
    pub format: ArtefactFormat,
    /// A short kebab-case name, used in prose and in nothing machine-readable.
    pub slug: &'static str,
    /// What the area is, in one line — the index's `Area` column.
    pub title: &'static str,
    /// How much judgement the area involves; compared against the index.
    pub risk: Risk,
    /// Builds the authored artefact from a `blank` document. Reads nothing.
    pub authored: fn() -> Result<Vec<u8>>,
    /// Builds the edited artefact from an Office-authored original.
    pub edit: fn(&[u8]) -> Result<Vec<u8>>,
}

impl Area {
    /// The file name of one of this area's artefacts, e.g. `v-pptx-01-authored.pptx`.
    #[must_use]
    pub fn artefact_name(&self, variant: Variant) -> String {
        format!(
            "{}-{}.{}",
            self.id.to_ascii_lowercase(),
            variant.label(),
            self.format.extension()
        )
    }

    /// The two-digit position within its format, as the index and `--area` both spell it.
    #[must_use]
    pub fn number(&self) -> &'static str {
        // `V-PPTX-01` -> `01`. The ids are compile-time constants of exactly this shape, and
        // `the_area_ids_are_well_formed` in `xtask/tests/validation_harness.rs` holds them to it.
        let (_, number) = self.id.rsplit_once('-').unwrap_or(("", self.id));
        number
    }
}

/// Every area, in the order the report and the index both use.
///
/// **This table is the generator half of the index gate.** The documented half is
/// `docs/validation/01-index.md`, hand-written; `xtask/tests/validation_index.rs` compares the two
/// in both directions and fails when either loses a row. Neither is generated from the other, which
/// is the whole point — an index test that compares two lists produced from one source proves
/// nothing.
pub static AREAS: &[Area] = &[
    Area {
        id: "V-PPTX-01",
        format: ArtefactFormat::Presentation,
        slug: "text-inheritance",
        title: "Text, paragraph and run properties, and what they inherit",
        risk: Risk::High,
        authored: presentation::authored_text_inheritance,
        edit: presentation::edit_text_inheritance,
    },
    Area {
        id: "V-PPTX-02",
        format: ArtefactFormat::Presentation,
        slug: "shape-appearance",
        title: "Preset geometry, fills, outlines and effects",
        risk: Risk::Medium,
        authored: presentation::authored_shape_appearance,
        edit: presentation::edit_shape_appearance,
    },
    Area {
        id: "V-PPTX-03",
        format: ArtefactFormat::Presentation,
        slug: "tables",
        title: "Tables: style parts, merges, cell fills, borders and margins",
        risk: Risk::High,
        authored: presentation::authored_tables,
        edit: presentation::edit_tables,
    },
    Area {
        id: "V-PPTX-04",
        format: ArtefactFormat::Presentation,
        slug: "charts",
        title: "Charts: series, axes, legend, data labels and trendlines",
        risk: Risk::Medium,
        authored: presentation::authored_charts,
        edit: presentation::edit_charts,
    },
    Area {
        id: "V-PPTX-05",
        format: ArtefactFormat::Presentation,
        slug: "pictures",
        title: "Pictures and picture fills",
        risk: Risk::Low,
        authored: presentation::authored_pictures,
        edit: presentation::edit_pictures,
    },
    Area {
        id: "V-PPTX-06",
        format: ArtefactFormat::Presentation,
        slug: "notes-and-links",
        title: "Speaker notes, shape hyperlinks and run hyperlinks",
        risk: Risk::Low,
        authored: presentation::authored_notes_and_links,
        edit: presentation::edit_notes_and_links,
    },
    Area {
        id: "V-PPTX-07",
        format: ArtefactFormat::Presentation,
        slug: "geometry",
        title: "Preset adjustments, custom geometry, and a 4:3 deck's rescaled placeholders",
        risk: Risk::Medium,
        authored: presentation::authored_geometry,
        edit: presentation::edit_geometry,
    },
    Area {
        id: "V-PPTX-08",
        format: ArtefactFormat::Presentation,
        slug: "chart-decoration",
        title: "Data labels, per-point formatting, trendlines, error bars and every plot type",
        risk: Risk::Medium,
        authored: presentation::authored_chart_decoration,
        edit: presentation::edit_chart_decoration,
    },
    Area {
        id: "V-DOCX-01",
        format: ArtefactFormat::Document,
        slug: "text-and-inheritance",
        title: "Paragraphs, runs, and the character properties they inherit",
        risk: Risk::High,
        authored: document::authored_text_and_inheritance,
        edit: document::edit_text_and_inheritance,
    },
    Area {
        id: "V-DOCX-02",
        format: ArtefactFormat::Document,
        slug: "sections-and-headers",
        title: "Section page size and margins, headers and footers",
        risk: Risk::Medium,
        authored: document::authored_sections_and_headers,
        edit: document::edit_sections_and_headers,
    },
    Area {
        id: "V-DOCX-03",
        format: ArtefactFormat::Document,
        slug: "tables",
        title: "Tables: horizontal spans and vertical merges",
        risk: Risk::High,
        authored: document::authored_tables,
        edit: document::edit_tables,
    },
    Area {
        id: "V-DOCX-04",
        format: ArtefactFormat::Document,
        slug: "charts",
        title: "Inline and floating charts, and their embedded workbooks",
        risk: Risk::Medium,
        authored: document::authored_charts,
        edit: document::edit_charts,
    },
    Area {
        id: "V-DOCX-05",
        format: ArtefactFormat::Document,
        slug: "pictures",
        title: "Inline pictures",
        risk: Risk::Low,
        authored: document::authored_pictures,
        edit: document::edit_pictures,
    },
    Area {
        id: "V-DOCX-06",
        format: ArtefactFormat::Document,
        slug: "notes-comments-links",
        title: "Footnotes, endnotes, comments and hyperlinks",
        risk: Risk::Medium,
        authored: document::authored_notes_comments_links,
        edit: document::edit_notes_comments_links,
    },
    Area {
        id: "V-XLSX-01",
        format: ArtefactFormat::Workbook,
        slug: "cell-values",
        title: "Cell values of every kind, shared strings and sheet tabs",
        risk: Risk::Low,
        authored: workbook::authored_cell_values,
        edit: workbook::edit_cell_values,
    },
    Area {
        id: "V-XLSX-02",
        format: ArtefactFormat::Workbook,
        slug: "cell-formats",
        title: "The two style layers: direct `cellXfs` over a named `cellStyleXfs`",
        risk: Risk::High,
        authored: workbook::authored_cell_formats,
        edit: workbook::edit_cell_formats,
    },
    Area {
        id: "V-XLSX-03",
        format: ArtefactFormat::Workbook,
        slug: "grid",
        title: "Merged ranges, row heights, column widths, hiding and outline levels",
        risk: Risk::Medium,
        authored: workbook::authored_grid,
        edit: workbook::edit_grid,
    },
    Area {
        id: "V-XLSX-04",
        format: ArtefactFormat::Workbook,
        slug: "charts",
        title: "Range charts anchored to a worksheet",
        risk: Risk::Medium,
        authored: workbook::authored_charts,
        edit: workbook::edit_charts,
    },
    Area {
        id: "V-XLSX-05",
        format: ArtefactFormat::Workbook,
        slug: "drawings",
        title: "Two-cell and one-cell anchored pictures, and their resizing behaviour",
        risk: Risk::Medium,
        authored: workbook::authored_drawings,
        edit: workbook::edit_drawings,
    },
    Area {
        id: "V-XLSX-06",
        format: ArtefactFormat::Workbook,
        slug: "comments-and-links",
        title: "Cell comments and cell hyperlinks",
        risk: Risk::Low,
        authored: workbook::authored_comments_and_links,
        edit: workbook::edit_comments_and_links,
    },
];

/// The default output directory, relative to the workspace root.
pub const DEFAULT_OUTPUT_DIRECTORY: &str = "target/validation-artefacts";

/// The workspace root — `xtask/..`.
#[must_use]
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// What the command was asked to do.
struct Options {
    format: Option<ArtefactFormat>,
    area: Option<String>,
    out: PathBuf,
    list: bool,
    /// Files handed in for ingestion rather than areas to generate. **The other direction**: every
    /// other option produces artefacts for a person to open, this one takes a file that person
    /// saved out of Office and reports what it is and what it holds. See [`ingest`].
    ingest: Vec<PathBuf>,
}

fn parse_options(arguments: &[String]) -> Result<Options> {
    let mut options = Options {
        format: None,
        area: None,
        out: workspace_root().join(DEFAULT_OUTPUT_DIRECTORY),
        list: false,
        ingest: Vec::new(),
    };
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--format" => {
                let value = rest
                    .next()
                    .context("--format needs a value: pptx, docx or xlsx")?;
                options.format = Some(ArtefactFormat::parse(value).with_context(|| {
                    format!("unknown format {value:?}. Available: pptx, docx, xlsx")
                })?);
            }
            "--area" => {
                let value = rest
                    .next()
                    .context("--area needs a value: an entry id like V-PPTX-03, or just 3")?;
                options.area = Some(value.to_ascii_uppercase());
            }
            "--out" => {
                let value = rest.next().context("--out needs a directory")?;
                options.out = PathBuf::from(value);
            }
            "--list" => options.list = true,
            "--ingest" => {
                let value = rest
                    .next()
                    .context("--ingest needs the path of a file saved out of Office")?;
                options.ingest.push(PathBuf::from(value));
            }
            other => bail!(
                "unknown argument {other:?}. Usage: validation-artefacts [--format pptx|docx|xlsx] \
                 [--area <id or number>] [--out <dir>] [--list] [--ingest <file>]"
            ),
        }
    }
    Ok(options)
}

/// Whether an area is one the options asked for.
///
/// `--area` accepts the whole id (`V-PPTX-03`), the bare number (`3` or `03`), and is combined with
/// `--format` by intersection, so `--format docx --area 3` is `V-DOCX-03` and `--area 3` alone is
/// that position in all three formats.
fn selected(area: &Area, options: &Options) -> bool {
    if let Some(format) = options.format {
        if area.format != format {
            return false;
        }
    }
    match options.area.as_deref() {
        None => true,
        Some(wanted) => {
            wanted == area.id
                || wanted.trim_start_matches('0') == area.number().trim_start_matches('0')
        }
    }
}

/// One line of the report: what happened to one (area, variant) pair.
enum Outcome {
    /// The artefact was written, with its size in bytes.
    Written { name: String, bytes: usize },
    /// The edit variant had no Office-authored original to edit. **Named, never silent.**
    SkippedNoOriginal { looked_for: PathBuf },
}

/// Runs the command.
///
/// # Errors
/// If an argument is unknown, a generator fails, the output cannot be written, or
/// `MJX_REQUIRE_OFFICE_CORPUS` is set and an edit variant had no original.
pub fn run(arguments: &[String]) -> Result<()> {
    let options = parse_options(arguments)?;
    if options.list {
        list();
        return Ok(());
    }
    if !options.ingest.is_empty() {
        return run_ingest(&options);
    }

    let chosen: Vec<&Area> = AREAS.iter().filter(|a| selected(a, &options)).collect();
    if chosen.is_empty() {
        bail!("no area matched --format/--area; `--list` prints the catalogue");
    }

    std::fs::create_dir_all(&options.out)
        .with_context(|| format!("creating {}", options.out.display()))?;
    println!(
        "validation-artefacts: {} area(s) -> {}",
        chosen.len(),
        options.out.display()
    );

    let mut written = 0usize;
    let mut skipped: Vec<(&str, PathBuf)> = Vec::new();
    for area in chosen {
        for variant in Variant::all() {
            let outcome = produce(area, variant, &options.out)?;
            match outcome {
                Outcome::Written { name, bytes } => {
                    written += 1;
                    println!(
                        "  {:<10} {:<9} {:<34} {bytes:>9} bytes",
                        area.id,
                        variant.label(),
                        name
                    );
                }
                Outcome::SkippedNoOriginal { looked_for } => {
                    println!(
                        "  {:<10} {:<9} SKIPPED — no Office-authored original at {}",
                        area.id,
                        variant.label(),
                        looked_for.display()
                    );
                    skipped.push((area.id, looked_for));
                }
            }
        }
    }

    println!(
        "validation-artefacts: {written} artefact(s) written, {} edit variant(s) skipped by name",
        skipped.len()
    );
    // MJX-ESCAPE-UNSET: this escape is deliberately bound by no workflow. The Office-authored
    // corpus at `tests/office-authored/` ships empty and no agent may fill it — a file's value
    // there is entirely its provenance — so setting it in CI would make the build red about
    // something no build can fix. Set it here once a person has run Office. (The marker is what
    // `xtask/tests/escape_hatches.rs` reads: every escape must be bound by a workflow or explained
    // at a definition site, because one that is neither is a suite reporting coverage it does not
    // have — MJXOFF-197.)
    if !skipped.is_empty() && std::env::var_os("MJX_REQUIRE_OFFICE_CORPUS").is_some() {
        bail!(
            "MJX_REQUIRE_OFFICE_CORPUS is set and {} area(s) had no Office-authored original: {}",
            skipped.len(),
            skipped
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if !skipped.is_empty() {
        println!(
            "note: the Office-authored corpus at {} is MJXOFF-130's to fill. Set \
             MJX_REQUIRE_OFFICE_CORPUS=1 to make its absence a failure.",
            corpus_directory().display()
        );
    }
    Ok(())
}

/// Reports what one or more Office-saved files are, and what every check says about them.
///
/// **Nothing is copied into the corpus.** The report says where each file *would* live; putting it
/// there is a person's decision, taken against the redistribution rule in
/// `tests/office-authored/README.md`, and a command that filed the file itself would be taking that
/// decision for them.
///
/// A failing check exits non-zero, so the command is usable in a script; a *reported* finding does
/// not, because it is a statement about a file this project did not write. `ingest.rs` draws that
/// line and says why.
///
/// # Errors
/// If a file cannot be read or classified, or any check failed.
fn run_ingest(options: &Options) -> Result<()> {
    let mut failed = 0usize;
    for path in &options.ingest {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .with_context(|| format!("{} names no file", path.display()))?;
        // `--area` overrides what the name implies, so a file straight out of Office can be reported
        // against the entry it answers before it is renamed.
        let area = match options.area.as_deref() {
            None => None,
            Some(token) => {
                let format = ingest::format_for_file_name(&name).with_context(|| {
                    format!("{name} has no extension this pass covers: .pptx, .docx or .xlsx")
                })?;
                Some(ingest::area_for_token(token, format).with_context(|| {
                    format!(
                        "--area {token} names no {} area; `--list` prints the catalogue",
                        format.extension()
                    )
                })?)
            }
        };
        let report = ingest::ingest(path, area)?;
        print!("{}", report.render());
        failed += report.failures().len();
    }
    println!(
        "validation-artefacts --ingest: {} file(s), {failed} failing check(s)",
        options.ingest.len()
    );
    if failed > 0 {
        bail!(
            "{failed} check(s) failed. A failing check is about *this library*, never about the \
             file: a package defect the file arrived with, or markup its producer wrote that the \
             XSDs reject, is reported rather than failed"
        );
    }
    Ok(())
}

/// Builds and writes one artefact, or reports why it could not be built.
fn produce(area: &Area, variant: Variant, out: &Path) -> Result<Outcome> {
    let bytes = match variant {
        Variant::Authored => {
            (area.authored)().with_context(|| format!("{} {}", area.id, variant.label()))?
        }
        Variant::Edited => match original_for(area)? {
            None => {
                return Ok(Outcome::SkippedNoOriginal {
                    looked_for: corpus::original_path(area),
                })
            }
            Some(original) => (area.edit)(&original)
                .with_context(|| format!("{} {}", area.id, variant.label()))?,
        },
    };
    let name = area.artefact_name(variant);
    let path = out.join(&name);
    std::fs::write(&path, &bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(Outcome::Written {
        name,
        bytes: bytes.len(),
    })
}

/// Prints the catalogue and stops. Useful on its own, and it is how a reader finds an area's id
/// without opening this file.
fn list() {
    println!(
        "{:<10}  {:<6}  {:<7}  {:<24}  title",
        "entry", "format", "risk", "area"
    );
    for area in AREAS {
        println!(
            "{:<10}  {:<6}  {:<7}  {:<24}  {}",
            area.id,
            area.format.extension(),
            area.risk.label(),
            area.slug,
            area.title
        );
    }
}
