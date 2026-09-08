//! Schema code generation: parse the local `References/` XSDs and emit committed Rust source into
//! `mjx-ooxml-types`. Deterministic — re-running produces no diff.

// `pub` rather than private since MJXOFF-224: this module tree moved into `xtask`'s library target
// so `xtask/tests/codegen_drift.rs` can be written against the generator's own tables, and a table
// whose field types are private cannot be named from a test. `xtask` is host-only, `publish = false`
// and nothing depends on it, so widening this widens no shipped surface.
pub mod child_order;
pub mod complex;
pub mod emit;
pub mod geometry;
pub mod namespaces;
pub mod naming;
pub mod spec;
pub mod xsd;

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

use crate::codegen::naming::NameEngine;

const STRICT_DIR: &str =
    "References/ECMA-376-1_5th_edition_december_2016/OfficeOpenXML-XMLSchema-Strict";
const TRANSITIONAL_DIR: &str =
    "References/ECMA-376-4_5th_edition_december_2016/OfficeOpenXML-XMLSchema-Transitional";
const GEOMETRIES_XML: &str =
    "References/ECMA-376-1_5th_edition_december_2016/OfficeOpenXML-DrawingMLGeometries/presetShapeDefinitions.xml";

/// One file this generator owns: where it belongs under the workspace root, and its exact
/// contents.
///
/// Generation and *writing* are separate so that the same bytes can be compared against what is
/// committed instead of overwriting it — see [`check`]. Nothing else re-derives this crate, so
/// without that separation a generator defect is frozen into the repository rather than failing on
/// the next build, and the committed file (the only artefact anyone reads) makes a defect
/// indistinguishable from a deliberate choice.
#[derive(Debug)]
pub struct Artefact {
    /// Absolute path of the committed file.
    pub path: PathBuf,
    /// Exactly the bytes that file should hold, rustfmt already applied where it applies.
    pub contents: String,
}

/// Whether the local `References/` tree holds everything this generator reads.
///
/// Three trees are needed and two of them are ECMA-376 **Part 1**, which
/// `.github/scripts/fetch-ecma-schemas.sh` does not download — its `ARCHIVES` holds Part 4
/// (Transitional) and Part 2 (OPC) only. That is why [`check`] is a local-or-gated check rather
/// than a CI job; see `crates/mjx-ooxml-types/docs/guide/what_to_distrust.md`.
#[must_use]
pub fn references_are_present(root: &Path) -> bool {
    root.join(STRICT_DIR).is_dir()
        && root.join(TRANSITIONAL_DIR).is_dir()
        && root.join(GEOMETRIES_XML).is_file()
}

/// Regenerates the `mjx-ooxml-types` source from the reference schemas.
pub fn run() -> Result<()> {
    let root = workspace_root();
    let artefacts = artefacts(&root)?;
    for artefact in &artefacts {
        if let Some(parent) = artefact.path.parent() {
            std::fs::create_dir_all(parent).context("creating generated/ dir")?;
        }
        write_plain(&artefact.path, &artefact.contents)?;
    }
    println!("codegen: wrote {}", artefact_list(&artefacts));
    Ok(())
}

/// Reports whether the committed output is what this generator produces today, and writes nothing.
///
/// This is the answer to the question the committed-output decision leaves open. It needs the same
/// local `References/` tree `run` does — see [`references_are_present`].
pub fn check() -> Result<()> {
    let root = workspace_root();
    let artefacts = artefacts(&root)?;
    let mut stale = Vec::new();
    for artefact in &artefacts {
        let committed = std::fs::read_to_string(&artefact.path)
            .with_context(|| format!("reading committed {}", artefact.path.display()))?;
        if committed != artefact.contents {
            stale.push(display_path(&root, &artefact.path));
        }
    }
    if !stale.is_empty() {
        bail!(
            "{} of {} generated artefact(s) differ from what the generator produces today:\n  {}\n\
             Run `cargo run -p xtask -- codegen` and commit the result.",
            stale.len(),
            artefacts.len(),
            stale.join("\n  ")
        );
    }
    println!(
        "codegen --check: {} artefact(s) match the generator: {}",
        artefacts.len(),
        artefact_list(&artefacts)
    );
    Ok(())
}

/// Renders every committed artefact, in emission order. Pure apart from reading `References/`.
pub fn artefacts(root: &Path) -> Result<Vec<Artefact>> {
    let strict_dir = root.join(STRICT_DIR);
    let transitional_dir = root.join(TRANSITIONAL_DIR);
    if !transitional_dir.is_dir() {
        bail!(
            "reference schemas not found at {} — codegen needs the local References/ tree",
            transitional_dir.display()
        );
    }

    let out_dir = root.join("crates/mjx-ooxml-types/src/generated");
    let mut out = Vec::new();

    // 1. namespace table (both worlds)
    let ns_src = namespaces::generate(&strict_dir, &transitional_dir)?;
    out.push(rust_artefact(out_dir.join("namespaces.rs"), &ns_src)?);

    // 2–3. the simple-type modules: one per schema, each with its own naming tables.
    let mut modules: Vec<(&SimpleTypeModule, emit::EmittedModule)> = Vec::new();
    for module in SIMPLE_TYPE_MODULES {
        let path = transitional_dir.join(format!("{}.xsd", module.stem));
        let xsd = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let source_note = format!(
            "References/.../OfficeOpenXML-XMLSchema-Transitional/{}.xsd",
            module.stem
        );
        let mut emitted = emit::emit_types(
            &xsd,
            &source_note,
            module.module_doc,
            module.engine,
            module.selection,
        )
        .with_context(|| format!("emitting {}", module.module))?;

        if module.module == "drawingml" {
            // The per-shape adjustment table, extracted from the DrawingML geometry definitions
            // rather than from a schema. It belongs beside `PresetShapeType`, whose variants key it.
            let geometries_path = root.join(GEOMETRIES_XML);
            let geometries_xml = std::fs::read(&geometries_path)
                .with_context(|| format!("reading {}", geometries_path.display()))?;
            emitted.source.push('\n');
            emitted
                .source
                .push_str(&geometry::emit_shape_adjustments(&geometries_xml)?);
        }

        out.push(rust_artefact(
            out_dir.join(format!("{}.rs", module.module)),
            &emitted.source,
        )?);
        modules.push((module, emitted));
    }

    // Every naming-override row must have matched something. See `spec::unused_overrides`.
    check_overrides_are_live(&modules)?;

    // 3c. child-order tables — the xsd:sequence position of every child of every complex type in
    //     the schemas this workspace authors markup for. Unlike the simple types there is no
    //     allowlist: the whole schema is emitted, because a serializer can only be prevented from
    //     writing out of sequence if the type it is writing is in the table.
    let mut schemas = Vec::new();
    for stem in CHILD_ORDER_SCHEMAS
        .iter()
        .chain(CHILD_ORDER_SCHEMA_DEPENDENCIES)
    {
        let path = transitional_dir.join(format!("{stem}.xsd"));
        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        schemas.push(complex::parse(&format!("{stem}.xsd"), &bytes)?);
    }
    let set = complex::SchemaSet::new(schemas);
    let child_order_src = child_order::generate(&set, CHILD_ORDER_SCHEMAS)?;
    out.push(rust_artefact(
        out_dir.join("child_order.rs"),
        &child_order_src,
    )?);

    // 4. the generated module root — derived from the module table, so a new module cannot be
    //    emitted and left unreachable.
    out.push(rust_artefact(
        out_dir.join("mod.rs"),
        &generated_module_root(),
    )?);

    // 5. coverage manifest (which schemas are generated vs pending), over every schema in the set
    let stems = namespaces::schema_stems(&transitional_dir)?;
    out.push(Artefact {
        path: root.join("crates/mjx-ooxml-types/COVERAGE.md"),
        contents: coverage_manifest(&stems, &modules)?,
    });

    Ok(out)
}

/// The artefacts' file names, in emission order — what `run` and `check` print.
fn artefact_list(artefacts: &[Artefact]) -> String {
    artefacts
        .iter()
        .filter_map(|a| a.path.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(", ")
}

/// A path relative to the workspace root, for a message a person reads.
fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// One generated simple-type module: which schema it reads, which types it takes, and the naming
/// tables it is named by.
///
/// This is the single list the emission, the generated `mod.rs` and the coverage manifest are all
/// driven from, so a schema cannot be generated without appearing in the manifest, and cannot
/// appear in the manifest claiming coverage it does not have.
#[derive(Debug)]
pub struct SimpleTypeModule {
    /// The schema file stem in the Transitional set, e.g. `wml`.
    pub stem: &'static str,
    /// The generated module (and file) name, e.g. `wordprocessingml`.
    pub module: &'static str,
    /// `pub` when the crate re-exports the module whole; `pub(crate)` when a hand-written module
    /// re-exports it item by item, so the public surface stays curated.
    pub visibility: &'static str,
    /// The module's `//!` doc block.
    pub module_doc: &'static str,
    /// The naming tables. `ST_*` symbols are schema-scoped, so this is per schema.
    pub engine: &'static NameEngine,
    /// Every type, or the curated slice.
    pub selection: emit::Selection<'static>,
}

/// The generated simple-type modules, in emission order.
pub const SIMPLE_TYPE_MODULES: &[SimpleTypeModule] = &[
    SimpleTypeModule {
        stem: "shared-commonSimpleTypes",
        module: "shared",
        visibility: "pub",
        module_doc: emit::SHARED_MODULE_DOC,
        engine: &spec::ENGINE,
        selection: emit::Selection::Everything,
    },
    SimpleTypeModule {
        stem: "dml-main",
        module: "drawingml",
        visibility: "pub(crate)",
        module_doc: emit::DRAWINGML_MODULE_DOC,
        engine: &spec::ENGINE,
        selection: emit::Selection::Allowlist(DRAWINGML_TYPES),
    },
    SimpleTypeModule {
        stem: "pml",
        module: "presentationml",
        visibility: "pub(crate)",
        module_doc: emit::PRESENTATIONML_MODULE_DOC,
        engine: &spec::ENGINE,
        selection: emit::Selection::Allowlist(PRESENTATIONML_TYPES),
    },
    SimpleTypeModule {
        stem: "wml",
        module: "wordprocessingml",
        visibility: "pub",
        module_doc: emit::WORDPROCESSINGML_MODULE_DOC,
        engine: &spec::WORDPROCESSINGML_ENGINE,
        selection: emit::Selection::Everything,
    },
    SimpleTypeModule {
        stem: "sml",
        module: "spreadsheetml",
        visibility: "pub",
        module_doc: emit::SPREADSHEETML_MODULE_DOC,
        engine: &spec::SPREADSHEETML_ENGINE,
        selection: emit::Selection::Everything,
    },
    SimpleTypeModule {
        stem: "shared-math",
        module: "officemath",
        visibility: "pub",
        module_doc: emit::OFFICEMATH_MODULE_DOC,
        engine: &spec::OFFICEMATH_ENGINE,
        selection: emit::Selection::Everything,
    },
    SimpleTypeModule {
        stem: "dml-diagram",
        module: "diagram",
        visibility: "pub",
        module_doc: emit::DIAGRAM_MODULE_DOC,
        engine: &spec::DIAGRAM_ENGINE,
        selection: emit::Selection::Everything,
    },
    SimpleTypeModule {
        stem: "dml-wordprocessingDrawing",
        module: "wordprocessingdrawing",
        visibility: "pub",
        module_doc: emit::WORDPROCESSINGDRAWING_MODULE_DOC,
        engine: &spec::ENGINE,
        selection: emit::Selection::Everything,
    },
    SimpleTypeModule {
        stem: "dml-spreadsheetDrawing",
        module: "spreadsheetdrawing",
        visibility: "pub",
        module_doc: emit::SPREADSHEETDRAWING_MODULE_DOC,
        engine: &spec::ENGINE,
        selection: emit::Selection::Everything,
    },
];

/// Fails if any naming-override row matched nothing across the modules its engine names.
fn check_overrides_are_live(modules: &[(&SimpleTypeModule, emit::EmittedModule)]) -> Result<()> {
    let mut engines: Vec<&'static NameEngine> = Vec::new();
    for (module, _) in modules {
        if !engines.iter().any(|e| std::ptr::eq(*e, module.engine)) {
            engines.push(module.engine);
        }
    }
    for engine in engines {
        let emitted: Vec<_> = modules
            .iter()
            .filter(|(m, _)| std::ptr::eq(m.engine, engine))
            .flat_map(|(_, e)| e.types.iter().cloned())
            .collect();
        let dead = spec::unused_overrides(engine, &emitted);
        if !dead.is_empty() {
            bail!(
                "naming table has {} row(s) that matched nothing:\n  {}",
                dead.len(),
                dead.join("\n  ")
            );
        }
    }
    Ok(())
}

/// Renders `generated/mod.rs` from [`SIMPLE_TYPE_MODULES`].
fn generated_module_root() -> String {
    let mut s = String::from(
        "// @generated by xtask — do not edit.\n\
         //! Generated OOXML type modules. Regenerate with `cargo run -p xtask -- codegen`.\n\n\
         // A `pub(crate)` module is re-exported item by item through a hand-written module of the\n\
         // same name, so the crate's public surface stays curated; a `pub` one is complete and is\n\
         // re-exported whole.\n\
         pub(crate) mod child_order;\n\
         pub mod namespaces;\n",
    );
    let mut rows: Vec<(&str, &str)> = SIMPLE_TYPE_MODULES
        .iter()
        .map(|m| (m.module, m.visibility))
        .collect();
    rows.sort_unstable();
    for (module, visibility) in rows {
        let _ = writeln!(s, "{visibility} mod {module};");
    }
    s
}

/// The schemas whose complex-type child orders are generated: every schema this workspace builds
/// markup in. They are parsed together so cross-schema `xsd:group` and `xsd:element` references
/// (PresentationML's use of DrawingML's fill and effect groups) resolve. A schema joins this list
/// when a crate starts authoring its markup — WordprocessingML and SpreadsheetML with Phases C/D.
///
/// `dml-wordprocessingDrawing` joined with MJXOFF-131 (C16): `mjx-dml::wordprocessing_drawing`
/// writes `wp:inline`/`wp:anchor`/the wrap modes/`wp:graphicFrame` from a typed model rather than a
/// fixed template, so every part that carries one is now ordered by construction too.
///
/// `shared-math` joined with MJXOFF-134 (C17): `mjx-omml` models all 72 `shared-math.xsd` complex
/// types and writes `m:oMath`/`m:oMathPara`/every math object from a typed model, so a part carrying
/// an equation is now ordered by construction too. It **left**
/// [`CHILD_ORDER_SCHEMA_DEPENDENCIES`] to get a table of its own here — exactly the move that
/// list's own doc comment describes for `dml-wordprocessingDrawing`.
///
/// `dml-spreadsheetDrawing` joined with MJXOFF-107 (E3), and **left**
/// [`CHILD_ORDER_SCHEMA_DEPENDENCIES`] to do it — the third schema to make that move, after
/// `dml-wordprocessingDrawing` and `shared-math`. It was parsed as a dependency from MJXOFF-132
/// onward because `sml.xsd`'s `CT_ObjectAnchor` places `xdr:from`/`xdr:to` by element `ref`; it now
/// has a table of its own because `mjx-dml::spreadsheet_drawing` writes `xdr:wsDr` and all three
/// anchor modes from a typed model, and `CT_TwoCellAnchor`'s own sequence
/// (`from`, `to`, the object choice, `clientData`) is an ordering no writer should be spelling out
/// by hand.
pub const CHILD_ORDER_SCHEMAS: &[&str] = &[
    "dml-main",
    "pml",
    "dml-chart",
    "dml-diagram",
    "wml",
    "dml-wordprocessingDrawing",
    "shared-math",
    "sml",
    "dml-spreadsheetDrawing",
];

/// Schemas parsed *only* to resolve a cross-schema `xsd:group`/`xsd:element` reference reached
/// while flattening one of [`CHILD_ORDER_SCHEMAS`]'s own types — never given a table of their own
/// here. An unresolved `xsd:group ref` is a hard error (see [`complex::SchemaSet::group`]), and so
/// is a slot whose element lands in a namespace with no parsed schema at all (see
/// `child_order::render_table`) — a schema that is merely *referenced*, never walked into, would
/// silently need no entry here, but nothing upstream can tell the difference in advance, so every
/// namespace `wml.xsd` reaches is listed:
/// - `shared-customXmlSchemaProperties` — `CT_SchemaLibrary`'s reference references
///   `sl:schemaLibrary` by element `ref`.
/// - `dml-picture` — `dml-wordprocessingDrawing`'s own `CT_WordprocessingGroup`/
///   `CT_WordprocessingCanvas` reference `dpct:pic` by element `ref` (MJXOFF-131).
///
/// `dml-wordprocessingDrawing` **left** this list with MJXOFF-131 (C16), `shared-math` left it with
/// MJXOFF-134 (C17), and `dml-spreadsheetDrawing` left it with MJXOFF-107 (E3) — all three now have
/// a table of their own in `CHILD_ORDER_SCHEMAS` instead, which is what a schema graduating out of
/// "referenced only" looks like. `dml-spreadsheetDrawing` was here from MJXOFF-132 because
/// `sml.xsd`'s `CT_ObjectAnchor` places `xdr:from`/`xdr:to` by element `ref`; it is now parsed for
/// its own sake, and `sml`'s cross-schema reference resolves out of the same set either way.
///
/// Adding a schema here does **not** generate its own child-order table or flip its `COVERAGE.md`
/// status — that stays the decision of the child that starts authoring *its* markup, by adding it
/// to `CHILD_ORDER_SCHEMAS` instead.
pub const CHILD_ORDER_SCHEMA_DEPENDENCIES: &[&str] =
    &["shared-customXmlSchemaProperties", "dml-picture"];

/// The DrawingML simple types given comprehensive names so far (see `spec.rs` for the naming data).
///
/// `dml-main.xsd` declares hundreds; only the curated ones are emitted, and the list grows as the
/// DrawingML workstream ports each.
pub const DRAWINGML_TYPES: &[&str] = &[
    "ST_ShapeType",
    "ST_SchemeColorVal",
    "ST_PresetPatternVal",
    "ST_ColorSchemeIndex",
    "ST_LineCap",
    "ST_CompoundLine",
    "ST_PenAlignment",
    "ST_PresetLineDashVal",
    "ST_LineEndType",
    "ST_LineEndWidth",
    "ST_LineEndLength",
    "ST_PresetShadowVal",
    "ST_RectAlignment",
    "ST_BlendMode",
    "ST_TextUnderlineType",
    "ST_TextStrikeType",
    "ST_TextCapsType",
    "ST_TextAlignType",
    "ST_TextFontAlignType",
    "ST_TextTabAlignType",
    "ST_TextAutonumberScheme",
    "ST_TextAnchoringType",
    "ST_TextVerticalType",
    "ST_TextHorzOverflowType",
    // DrawingML table styles (`tableStyles.xml`): the tri-state a table style's bold/italic
    // take (`a:tcTxStyle@b`/`@i`), and the theme font slot a `a:fontRef` names.
    "ST_OnOffStyleType",
    "ST_FontCollectionIndex",
    // DrawingML 3-D (`a:scene3d` / `a:sp3d` / `a:cell3D`): the bevel and light-rig presets,
    // the light direction, the surface material, and the preset camera view.
    "ST_BevelPresetType",
    "ST_LightRigType",
    "ST_LightRigDirection",
    "ST_PresetMaterialType",
    "ST_PresetCameraType",
    // DrawingML custom geometry (`a:custGeom`): how a freeform `a:path` is filled.
    "ST_PathFillMode",
];

/// The PresentationML simple types given comprehensive names so far (see `spec.rs` for the naming
/// data). Layout/placeholder identity first — the slide-layout workstream's vocabulary.
pub const PRESENTATIONML_TYPES: &[&str] = &[
    "ST_PlaceholderType",
    "ST_PlaceholderSize",
    "ST_SlideLayoutType",
    "ST_SlideSizeType",
    "ST_Direction",
];

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is the xtask crate dir; the workspace root is its parent.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent dir")
        .to_path_buf()
}

/// A Rust artefact: the emitted source with rustfmt already applied, so that what `run` writes and
/// what `check` compares against are the same bytes rather than two spellings of them.
fn rust_artefact(path: PathBuf, src: &str) -> Result<Artefact> {
    let contents = rustfmt(src).with_context(|| format!("formatting {}", path.display()))?;
    Ok(Artefact { path, contents })
}

fn write_plain(path: &Path, contents: &str) -> Result<()> {
    std::fs::write(path, contents).with_context(|| format!("writing {}", path.display()))
}

fn rustfmt(src: &str) -> Result<String> {
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2021"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawning rustfmt (is the `rustfmt` component installed?)")?;
    child
        .stdin
        .take()
        .context("rustfmt stdin")?
        .write_all(src.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!(
            "rustfmt failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8(output.stdout).context("rustfmt produced non-UTF-8")
}

/// Renders `COVERAGE.md`: one row per schema in the Transitional set, for both generated tables.
///
/// Every column is **derived**. The simple-type status comes from [`SIMPLE_TYPE_MODULES`] and the
/// types that module actually emitted; the child-order status from [`CHILD_ORDER_SCHEMAS`]. A
/// schema in neither gets its curated note from [`UNCOVERED_SCHEMAS`], and a schema in *nothing* —
/// not generated, not noted — is a hard error, so a new schema file cannot arrive and be silently
/// absent from the document that reports coverage.
fn coverage_manifest(
    stems: &[String],
    modules: &[(&SimpleTypeModule, emit::EmittedModule)],
) -> Result<String> {
    // The notes are claims about schemas; a claim about a schema that is not there is a typo.
    for (i, (stem, _, _)) in UNCOVERED_SCHEMAS.iter().enumerate() {
        if !stems.iter().any(|s| s == stem) {
            bail!("`UNCOVERED_SCHEMAS` names `{stem}`, which is not in the Transitional set");
        }
        if UNCOVERED_SCHEMAS[..i].iter().any(|(s, _, _)| s == stem) {
            bail!("`UNCOVERED_SCHEMAS` names `{stem}` twice");
        }
    }

    let mut s = String::new();
    s.push_str("# Generated-type coverage\n\n");
    s.push_str("_Generated by `cargo run -p xtask -- codegen`; do not edit by hand._\n\n");
    s.push_str(
        "Every schema of the ECMA-376 Transitional set has a row in **both** tables, so a schema \
         can never be uncovered *and* unlisted. The status of each row is derived — from the \
         generator's module table and from `CHILD_ORDER_SCHEMAS` — never written down, so a later \
         change flips its own row.\n\n",
    );

    s.push_str(
        "## Simple types and constant tables\n\n| Schema | Module | Status |\n|---|---|---|\n",
    );
    for stem in stems {
        let (module, status) = match modules.iter().find(|(m, _)| m.stem == stem) {
            Some((module, emitted)) => {
                let status = match module.selection {
                    emit::Selection::Everything => format!(
                        "generated — all {} simple types",
                        emitted
                            .types
                            .iter()
                            .filter(|t| !spec::SKIP_TYPES.contains(&t.name.as_str()))
                            .count()
                    ),
                    emit::Selection::Allowlist(list) => {
                        let mut names: Vec<&str> = list.to_vec();
                        names.sort_unstable();
                        format!(
                            "partial ({} of the schema's simple types: {})",
                            names.len(),
                            names
                                .iter()
                                .map(|n| format!("`{n}`"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                };
                (format!("`{}`", module.module), status)
            }
            None => ("—".to_owned(), uncovered_note(stem, Table::SimpleTypes)?),
        };
        let _ = writeln!(s, "| {stem} | {module} | {status} |");
    }

    s.push_str("\n## Child order\n\n");
    s.push_str(
        "`child_order.rs` holds the `xsd:sequence` position of every child of **every** complex \
         type of the schemas below — no allowlist, because a serializer can only be stopped from \
         writing out of sequence if the type it is writing is in the table. A schema joins the list \
         when a crate starts authoring its markup.\n\n",
    );
    s.push_str("| Schema | Child order |\n|---|---|\n");
    for stem in stems {
        let status = if CHILD_ORDER_SCHEMAS.contains(&stem.as_str()) {
            "generated — every complex type".to_owned()
        } else {
            uncovered_note(stem, Table::ChildOrder)?
        };
        let _ = writeln!(s, "| {stem} | {status} |");
    }
    Ok(s)
}

/// Which of `COVERAGE.md`'s two tables a note is being written for.
#[derive(Debug, Clone, Copy)]
enum Table {
    SimpleTypes,
    ChildOrder,
}

/// The curated note for a schema a table does not cover.
fn uncovered_note(stem: &str, table: Table) -> Result<String> {
    let row = UNCOVERED_SCHEMAS
        .iter()
        .find(|(s, _, _)| *s == stem)
        .with_context(|| {
            format!(
                "schema `{stem}.xsd` is in the Transitional set but has no row in \
                 `UNCOVERED_SCHEMAS`, so `COVERAGE.md` cannot report it. Add a row saying who owns \
                 it, or why it will never be modelled."
            )
        })?;
    let note = match table {
        Table::SimpleTypes => row.1,
        Table::ChildOrder => row.2,
    };
    if note.is_empty() {
        bail!(
            "`{stem}.xsd` has an empty note for the {table:?} table, but that table does not cover \
             it — an empty note means \"covered here, see the other column\""
        );
    }
    Ok(note.to_owned())
}

/// Why a schema is not covered, per table: `(stem, simple-type note, child-order note)`.
///
/// A row is needed for every schema not generated in that table, and only for those: a schema that
/// gains coverage in **both** tables has its row removed (MJXOFF-132 took `sml`'s and the dead one
/// MJXOFF-134 left behind for `shared-math`), and a schema that arrives without one fails the
/// generator. Removing the row is not tidying — it is what makes coverage load-bearing: withdraw
/// `sml` from [`CHILD_ORDER_SCHEMAS`] now and this generator refuses to run rather than quietly
/// reporting a schema as pending again. `pending, owned by MJXOFF-N` names the work item that closes the gap — the same
/// ownership `mjx-schema-gate`'s `OrderingCoverage::Pending` states for the namespaces it
/// categorises, and `crates/mjx-schema-gate/src/categories.rs` has a test that the two agree.
pub const UNCOVERED_SCHEMAS: &[(&str, &str, &str)] = &[
    (
        "dml-chart",
        "pending — charts are written through `mjx-chart`'s model, which uses no `ST_*` \
         enumeration of its own yet",
        "generated — every complex type",
    ),
    (
        "dml-chartDrawing",
        "not modelled — the drawing canvas inside a chart part; nothing in this workspace authors \
         one",
        "not modelled — as for its simple types",
    ),
    (
        "dml-lockedCanvas",
        "not modelled — a compatibility wrapper with no simple types this workspace reads",
        "not modelled — nothing in this workspace authors a locked canvas",
    ),
    (
        "dml-picture",
        "not modelled — `pic:pic` carries DrawingML types, which `dml-main` already provides",
        "modelled by construction, not by table — `mjx-dml::picture::Picture` (MJXOFF-131) reads and \
         writes `CT_Picture`'s three children (`nvPicPr`, `blipFill`, `spPr`) in that one fixed \
         order every time; the schema gives this type no choice, no optional member and no repeated \
         member to rank, so there is no placement decision a generated table could inform that the \
         hand-written order does not already get right",
    ),
    ("pml", "", "generated — every complex type"),
    (
        "shared-additionalCharacteristics",
        "not modelled — a document-characteristics part this workspace neither reads nor writes",
        "not modelled — as for its simple types",
    ),
    (
        "shared-bibliography",
        "not modelled — bibliography sources are preserved verbatim, never authored",
        "not modelled — as for its simple types",
    ),
    (
        "shared-customXmlDataProperties",
        "not modelled — custom XML data parts are preserved verbatim",
        "not modelled — as for its simple types",
    ),
    (
        "shared-customXmlSchemaProperties",
        "not modelled — custom XML schema references are preserved verbatim",
        "not modelled — as for its simple types",
    ),
    (
        "shared-documentPropertiesCustom",
        "not modelled — `docProps/custom.xml` is preserved verbatim and never authored; MJXOFF-149 \
         (which authored core and extended document properties) deliberately left it out of scope: \
         no committed fixture carries one",
        "not modelled — as for its simple types",
    ),
    (
        "shared-documentPropertiesExtended",
        "not modelled — `CT_Properties` declares no `xsd:simpleType` beyond XSD primitives, so \
         there is nothing here for this generator to emit. `docProps/app.xml` **is** authored, by \
         hand, in `mjx_opc::doc_props::extended_xml` (MJXOFF-149) — see \
         `mjx-schema-gate::categories::MODELED_SCHEMAS`",
        "not modelled — `CT_Properties` is an `xs:all` group; ECMA-376 places no order constraint \
         on its children, so there is no `xsd:sequence` for this generator to place them by",
    ),
    (
        "shared-documentPropertiesVariantTypes",
        "not modelled — the variant vocabulary of the custom-properties part, preserved verbatim",
        "not modelled — as for its simple types",
    ),
    (
        "shared-relationshipReference",
        "not modelled — `r:id` is a token `mjx-opc` owns; the schema declares no enumeration",
        "not modelled — relationship references are attributes, not a content model",
    ),
    (
        "vml-main",
        "not modelled — VML is a legacy Microsoft vocabulary this project stores and re-emits \
         verbatim; `mjx-vml` never authors a `ST_*` value",
        "not modelled — VML parts are preserved byte for byte, never re-sequenced",
    ),
    (
        "vml-officeDrawing",
        "not modelled — as for `vml-main`",
        "not modelled — as for `vml-main`",
    ),
    (
        "vml-presentationDrawing",
        "not modelled — as for `vml-main`",
        "not modelled — as for `vml-main`",
    ),
    (
        "vml-spreadsheetDrawing",
        "not modelled — as for `vml-main`",
        "not modelled — as for `vml-main`",
    ),
    (
        "vml-wordprocessingDrawing",
        "not modelled — as for `vml-main`",
        "not modelled — as for `vml-main`",
    ),
    // `wml`'s child-order note is unused now that CHILD_ORDER_SCHEMAS contains it (its column is
    // computed directly, the same as `dml-main`'s row below) — kept accurate rather than stale, on
    // the same convention that row already follows.
    ("wml", "", "generated — every complex type"),
    ("dml-main", "", "generated — every complex type"),
    (
        "shared-commonSimpleTypes",
        "",
        "not modelled — the schema declares simple types only; it has no complex type to order",
    ),
];
