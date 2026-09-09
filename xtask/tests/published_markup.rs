//! **ECMA's own published markup as a third authority** (MJXOFF-250, items 2 and 3).
//!
//! # The source
//!
//! The ECMA-376 Part 1 5th-edition package is not only a PDF and a set of schemas. It also ships
//! five artefacts of *data*, and until MJXOFF-246 nothing in this repository had read any of them:
//!
//! | Artefact | What it is |
//! |---|---|
//! | `OfficeOpenXML-SpreadsheetMLStyles/presetTableStyles.xml` | the built-in table and pivot styles, as `dxf` bands |
//! | `OfficeOpenXML-SpreadsheetMLStyles/presetCellStyles.xml` | the built-in cell styles, one `styleSheet` each |
//! | `OfficeOpenXML-SpreadsheetMLStyles/PivotTableFormats.xlsx` | a whole workbook |
//! | `OfficeOpenXML-DrawingMLGeometries/presetShapeDefinitions.xml` | every preset shape's geometry |
//! | `OfficeOpenXML-DrawingMLGeometries/presetTextWarpDefinitions.xml` | every preset text warp's geometry |
//! | `OfficeOpenXML-WordprocessingMLArtBorders/*.png` | the art borders, as images |
//!
//! This is **markup published by the standard**. It settles questions the prose leaves ambiguous
//! without appealing to Microsoft, and it is not the Office corpus MJXOFF-130 is about — it needs
//! no provenance argument at all.
//!
//! # Item 2 — which of this project's tables are derivable from it, and which are not
//!
//! The ticket asked for *a written list*, explicitly **not** a promise to derive all of them. Here
//! it is. Every "no" below is a measurement rather than a preference, and the two that can be
//! checked are checked in this file, so a "no" that stops being true fails rather than sitting here.
//!
//! | This project's table | Artefact | Derivable? |
//! |---|---|---|
//! | `mjx_sml::styles::BuiltInTableStyleFamily`'s six families and six bounds | `presetTableStyles.xml` | **Yes.** Done — MJXOFF-250 item 1, in `crates/mjx-sml/tests/theme_index.rs`. All six agreed exactly. |
//! | `mjx_sml::styles::theme_color_position`'s twelve slots | `presetTableStyles.xml` + `presetCellStyles.xml` | **Yes.** Done — MJXOFF-246, same file, and it *contradicted* the inference the prose's cross-reference gave. |
//! | `mjx_ooxml_types::drawingml::PresetShapeType`'s wire tokens | `presetShapeDefinitions.xml` | **Yes, in one direction and all but one member of the other.** [`every_published_preset_geometry_names_a_shape_this_project_models`] below. |
//! | `mjx_sml::styles::builtin_cell_style_name` — Annex G.2's `builtinId` table | `presetCellStyles.xml` | **No**, and the reason is a defect in the artefact rather than a gap in this project: [`ecmas_preset_cell_styles_cannot_settle_the_builtin_id_table`]. |
//! | `mjx_sml::styles::borders`' `x:start` / `x:end` meaning | `presetCellStyles.xml`, `presetTableStyles.xml` | **No** — neither artefact writes either element even once. [`ecmas_published_markup_never_writes_a_logical_edge_border`]. |
//! | a preset **text warp** table | `presetTextWarpDefinitions.xml` | **No such table.** The artefact publishes 40 warps; this project models no text-warp enumeration at all, so there is nothing here to settle. |
//! | a **pivot table** model | `PivotTableFormats.xlsx` | **No such table.** `crates/mjx-sml/src/preserved/pivot.rs` *preserves* pivot parts rather than modelling them, so there is no table of ours for a workbook to check. |
//! | `mjx_ooxml_types::wordprocessingml`'s art-border tokens | `WordprocessingMLArtBorders/*.png` | **No** — the artefact is *file names*, not markup, and they are lossy: `palmsColo` is the token `palmsColor` truncated and `waveLine` is the token `waveline` re-cased. A derivation off them would have to guess at both. |
//!
//! The through-line: **an artefact settles a table when the artefact *is* the population.**
//! `presetTableStyles.xml` is the 144 preset names, so comparing is exact. `presetShapeDefinitions`
//! is the geometries, so it is exact for every shape that has one. `presetCellStyles.xml` is a set
//! of sixty-three one-style *workbooks*, and a workbook carries whatever its author saved — which is
//! how three of them came to disagree with the standard's own prose.
//!
//! # Item 3 — a spec *cross-reference* is not a spec *statement*
//!
//! MJXOFF-246's resolver followed §20.1.6.2's *Sequence Index* table because §18.8.19 pointed at
//! that clause, and the standard's own data contradicted the inference. The ticket asks for every
//! other place in this workspace of the same shape. This is that list, and each entry says what
//! kind of link was followed, because they are not equally strong.
//!
//! | Site | The link that was followed | Where it stands |
//! |---|---|---|
//! | `crates/mjx-sml/src/styles/palette.rs` — `@theme` positions | §18.8.19 → §20.1.6.2, an explicit cross-reference | **The instance.** MJXOFF-246 found the inference wrong and replaced it with a derivation from published data. |
//! | `crates/mjx-sml/src/styles/borders.rs` — `x:start` / `x:end` | **No link at all.** §18.8 has no entry for either element; the meaning is taken from WordprocessingML's §17.4.33 and §17.4.12, which document identically-named elements. | The **weakest** shape on this list — weaker than a cross-reference, because nothing in Part 1 connects the two clauses. It is a reading by analogy, and it is what the doc comments say it is. Published markup cannot settle it: [`ecmas_published_markup_never_writes_a_logical_edge_border`] measures that neither artefact writes either element. Nothing else can, short of an Office-authored file, so this stays a stated inference. |
//! | `crates/mjx-xlsx/src/parts.rs` — `REL_CUSTOM_PROPERTY`'s source part | §12.3's summary table says the **Workbook** part; §12.3.5's body and its example say the **Worksheet** part | **Already handled the right way**, and it is the counter-example worth keeping in view: the two statements are quoted rather than reconciled, the code follows the body, and the doc comment says every file this project has read agrees with the body. That is prose *plus evidence*, which is what MJXOFF-246's site lacked. |
//! | `crates/mjx-sml/src/worksheet/anchors.rs` — `BaseColumnWidth` | `@defaultColWidth`'s absence sends the derivation through §18.3.1.81's formula over `@baseColWidth` | A cross-reference to a **formula**, not to a table of meanings. Following a stated computation is not the same act as inferring a mapping from a neighbouring clause's ordering, and no data could contradict it the way MJXOFF-246's was contradicted. |
//! | `crates/mjx-dml/src/diagram/data.rs` — `OneByOneAnimation` | a citation that pointed at §21.4.6.2 and should have pointed at §21.4.6 | A mis-citation, already corrected in place and marked as such. Not an inference. |
//!
//! **What this sweep cannot see**, stated rather than left implicit: it is a sweep over what the
//! source *says*. It finds a site whose doc comment names two clauses and explains which one it
//! followed. A decision that followed a cross-reference and wrote nothing down is invisible to it,
//! and no scan can find one — which is the argument for the convention this repository already
//! keeps, that a clause is cited wherever it decided something.
//!
//! # The mutation register
//!
//! Every test below was made to fail by a reachable mutation; the verbatim output is in the pull
//! request for MJXOFF-250.
//!
//! | Mutation | Fails |
//! |---|---|
//! | delete `upArrow`'s row from [`SHAPES_WITH_NO_PUBLISHED_GEOMETRY`] | [`every_published_preset_geometry_names_a_shape_this_project_models`], on the set comparison |
//! | make `PresetShapeType::from_wire` refuse one published token | the same test, on the *published → modelled* direction, naming the token |
//! | delete `heading1`'s row from [`CELL_STYLE_ARTEFACT_DEFECTS`] | [`ecmas_preset_cell_styles_cannot_settle_the_builtin_id_table`], as an unregistered disagreement |
//! | give `builtin_cell_style_name` the artefact's `Heading 1` at 17 | the same test — and **not** as a healed row: taking the artefact's number for `Heading 1` leaves `Heading 2` mapping to nothing, so `heading2` becomes the unregistered disagreement. Following a defective artefact does not remove a disagreement, it moves it |
//! | make the border element counter match a name it never sees | [`ecmas_published_markup_never_writes_a_logical_edge_border`], on its physical-edge floor |
//!
//! # Skipping
//!
//! Every derivation here skips without `References/`, on the footing `mjx_schema_gate::harness` and
//! `crates/mjx-sml/tests/theme_index.rs` already use; `MJX_REQUIRE_SCHEMA=1` — which is what CI
//! sets — turns an absence into a failure. [`the_preset_shape_tokens_are_ordinary_words`] needs no
//! artefact at all and never skips.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use mjx_ooxml_types::drawingml::PresetShapeType;

// ===============================================================================================
// Locating the artefacts
// ===============================================================================================

/// The repository root — `xtask/..`.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// Reads a repository-relative file, failing loudly: an unreadable file must never become an empty
/// population.
fn read(relative: &str) -> String {
    let path = repository_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// One of ECMA's published artefacts, or `None` when the gate should skip.
///
/// Mirrors `crates/mjx-sml/tests/theme_index.rs`'s `preset_styles_dir` rather than inventing a
/// second convention: located under the git-ignored `References/` tree at the workspace root, and
/// `MJX_REQUIRE_SCHEMA` turns an absence into a failure rather than a skip.
///
/// # Panics
/// If `MJX_REQUIRE_SCHEMA` is set and the file is not there.
fn published(relative: &str, what: &str) -> Option<String> {
    const EDITION: &str = "ECMA-376-1_5th_edition_december_2016";
    let path = repository_root()
        .join("References")
        .join(EDITION)
        .join(relative);
    if path.is_file() {
        return Some(
            std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display())),
        );
    }
    assert!(
        std::env::var_os("MJX_REQUIRE_SCHEMA").is_none(),
        "MJX_REQUIRE_SCHEMA is set but {what} (References/{EDITION}/{relative}) could not be found \
         at {}",
        path.display()
    );
    eprintln!(
        "skipping {what}: ECMA's published markup is not on this machine \
         (References/{EDITION}/{relative})"
    );
    None
}

/// The names of a published definitions file's top-level children, in document order.
///
/// Both `presetShapeDefinitions.xml` and `presetTextWarpDefinitions.xml` have the same shape: a
/// root whose every child element *is* one definition, named by its own tag. Reading the tags with
/// a scan rather than a parser is deliberate — `mjx-xml` would do it properly, but the thing being
/// read is a list of element names at one depth and a scan cannot disagree with itself about that.
fn definition_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut depth = 0usize;
    let mut rest = source;
    while let Some(at) = rest.find('<') {
        rest = &rest[at + 1..];
        let Some(close) = rest.find('>') else { break };
        let tag = &rest[..close];
        rest = &rest[close + 1..];
        if tag.starts_with('?') || tag.starts_with('!') {
            continue;
        }
        if let Some(name) = tag.strip_prefix('/') {
            depth = depth.saturating_sub(1);
            let _ = name;
            continue;
        }
        let self_closing = tag.ends_with('/');
        let name: String = tag
            .trim_end_matches('/')
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_owned();
        if depth == 1 && !name.is_empty() {
            names.push(name);
        }
        if !self_closing {
            depth += 1;
        }
    }
    names
}

// ===============================================================================================
// The preset shape geometries
// ===============================================================================================

/// The relative path of the published geometry file.
const GEOMETRIES: &str = "OfficeOpenXML-DrawingMLGeometries/presetShapeDefinitions.xml";

/// Every wire token [`PresetShapeType`] accepts, read out of the generated `from_wire`.
///
/// Read out of the **parser's own match arms**, not out of the doc comments beside the variants:
/// the question here is which tokens the crate answers to, and the arms are that, while a doc
/// comment is a description of it.
fn preset_shape_wire_tokens() -> BTreeSet<String> {
    let source = read("crates/mjx-ooxml-types/src/generated/drawingml.rs");
    let start = source
        .find("impl PresetShapeType {")
        .expect("the generated module declares `impl PresetShapeType`");
    let from_wire = start
        + source[start..]
            .find("pub fn from_wire")
            .expect("`PresetShapeType` has a `from_wire`");
    let end = from_wire
        + source[from_wire..]
            .find("\n    }")
            .expect("`from_wire` closes");

    let mut tokens = BTreeSet::new();
    for line in source[from_wire..end].lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix('"') else {
            continue;
        };
        let Some((token, tail)) = rest.split_once('"') else {
            continue;
        };
        if tail.trim_start().starts_with("=>") {
            tokens.insert(token.to_owned());
        }
    }
    tokens
}

/// **Every geometry ECMA publishes names a shape this project models, and only one modelled shape
/// has no published geometry.**
///
/// The comparison runs in both directions and neither is decoration:
///
/// * *published → modelled* goes through [`PresetShapeType::from_wire`] itself and through
///   `to_wire` back again, so what is checked is the crate's own parser rather than a list beside
///   it. A published name the parser refuses fails here by name.
/// * *modelled → published* is the direction with an exception in it, and the exception is
///   **registered** rather than subtracted: [`SHAPES_WITH_NO_PUBLISHED_GEOMETRY`] names it, and a
///   token that stops needing to be there fails as loudly as one that starts.
///
/// Floors are phrased as *the scan has stopped matching*, never as a total, because ECMA may
/// reissue the artefact and the generator may regain a schema.
#[test]
fn every_published_preset_geometry_names_a_shape_this_project_models() {
    let Some(source) = published(GEOMETRIES, "the published preset geometries") else {
        return;
    };
    let published_names: BTreeSet<String> = definition_names(&source).into_iter().collect();
    assert!(
        published_names.len() > 100,
        "only {} published geometry name(s) were read out of {GEOMETRIES} — the scan has stopped \
         matching, and with nothing to compare this test passes exactly as a working one does",
        published_names.len()
    );

    let mut refused = Vec::new();
    for name in &published_names {
        match PresetShapeType::from_wire(name) {
            Some(shape) if shape.to_wire() == name => {}
            Some(shape) => refused.push(format!(
                "{name} parsed but writes back as {}",
                shape.to_wire()
            )),
            None => refused.push(format!("{name} is not a token `PresetShapeType` accepts")),
        }
    }
    assert!(
        refused.is_empty(),
        "{} name(s) ECMA publishes a geometry for are not shapes this project models:\n  {}\n\nThe \
         published geometries are the standard's own data. A name here that the crate refuses is a \
         preset shape a document may legitimately carry and this library cannot name.",
        refused.len(),
        refused.join("\n  ")
    );

    let tokens = preset_shape_wire_tokens();
    assert!(
        tokens.len() >= published_names.len(),
        "`PresetShapeType::from_wire` was read as accepting only {} token(s) against {} published \
         geometries — the match-arm scan has stopped matching",
        tokens.len(),
        published_names.len()
    );

    let unpublished: BTreeSet<&String> = tokens.difference(&published_names).collect();
    let registered: BTreeSet<&String> =
        SHAPES_WITH_NO_PUBLISHED_GEOMETRY
            .iter()
            .map(|(token, _)| {
                tokens.get(*token).unwrap_or_else(|| {
            panic!("`SHAPES_WITH_NO_PUBLISHED_GEOMETRY` names `{token}`, which is not a token \
                    `PresetShapeType` accepts at all — delete the row")
        })
            })
            .collect();
    assert_eq!(
        unpublished, registered,
        "the shapes this project models that ECMA publishes no geometry for are not the ones \
         `SHAPES_WITH_NO_PUBLISHED_GEOMETRY` names. A token that joined the left side is a shape \
         whose geometry the standard does not define and nobody wrote down why; a token that left \
         it means ECMA has reissued the artefact and the row should go."
    );

    println!(
        "preset geometries: {} published, all of them tokens `PresetShapeType` round-trips; {} \
         token(s) accepted with no published geometry, all registered: {:?}",
        published_names.len(),
        unpublished.len(),
        unpublished
    );
}

/// Tokens `PresetShapeType` accepts for which `presetShapeDefinitions.xml` publishes no geometry.
///
/// A register, not a subtraction: [`every_published_preset_geometry_names_a_shape_this_project_models`]
/// compares this against the real difference in both directions, so a row cannot outlive the gap it
/// names and a new gap cannot arrive unremarked.
const SHAPES_WITH_NO_PUBLISHED_GEOMETRY: &[(&str, &str)] = &[(
    "upArrow",
    "`dml-main.xsd`'s `ST_ShapeType` enumerates it and `presetShapeDefinitions.xml` has no \
     `<upArrow>` element. The artefact carries 187 children of which `<upDownArrow>` is written \
     twice, so it publishes 186 distinct geometries against the schema's 187 tokens — the one \
     missing is this. Nothing in this project depends on the geometry; `PresetShapeType` is a \
     token vocabulary, and a document naming `upArrow` is conforming markup either way.",
)];

/// **The preset shape tokens are ordinary short words, which is why they are not a roster
/// population.**
///
/// `xtask/tests/derived_rosters.rs` sweeps the workspace for literal lists that are the whole of
/// something this repository can enumerate, and the preset shape names were a candidate for its
/// `BasePopulation`. They were rejected, and this is the measurement the rejection rests on: a
/// population is only useful to that scanner when a list of its members is a roster **rather than
/// a coincidence**, and `ST_ShapeType` is full of plain English — `line`, `arc`, `home`, `plus`,
/// `pie`, `sun`, `moon`, `heart`, `frame`, `cube`, `can`. Any two of those written next to each
/// other for an unrelated reason would be read as a roster.
///
/// It lives here rather than beside the sweep because this file already owns the token set, and a
/// second reader of `PresetShapeType`'s tokens would be a second answer to one question. It needs
/// no artefact and never skips.
#[test]
fn the_preset_shape_tokens_are_ordinary_words() {
    let tokens = preset_shape_wire_tokens();
    assert!(
        tokens.len() >= 100,
        "only {} `PresetShapeType` wire token(s) were read out of `from_wire` — the match-arm scan \
         has stopped matching, so the measurement below means nothing. Read: {tokens:?}",
        tokens.len()
    );

    // "Plain" here means what would make a collision plausible: all lower-case letters, six
    // characters or fewer. `roundRect` is nobody's variable name; `frame` is everybody's.
    let plain: BTreeSet<&String> = tokens
        .iter()
        .filter(|token| token.len() <= 6 && token.chars().all(|c| c.is_ascii_lowercase()))
        .collect();
    assert!(
        plain.len() >= 20,
        "only {} of the {} `PresetShapeType` tokens are short plain lower-case words. The \
         objection the exclusion rests on — that a list of two of them is as likely a coincidence \
         as a roster — is weaker than it was, and `derived_rosters.rs`'s `BasePopulation` should be \
         reconsidered for them. The plain ones: {plain:?}",
        plain.len(),
        tokens.len()
    );

    println!(
        "PresetShapeType: {} wire token(s), of which {} are short plain lower-case words — the \
         reason the preset shape names are not a `derived_rosters.rs` population",
        tokens.len(),
        plain.len()
    );
}

// ===============================================================================================
// The preset cell styles — the artefact that cannot settle its table
// ===============================================================================================

/// The relative path of the published cell styles.
const CELL_STYLES: &str = "OfficeOpenXML-SpreadsheetMLStyles/presetCellStyles.xml";

/// Where `presetCellStyles.xml` and Annex G.2 disagree, and which one this project follows.
///
/// Each row is `(the artefact's own element name, what the artefact says, why Annex G.2 wins)`. A
/// register rather than an allowlist: [`ecmas_preset_cell_styles_cannot_settle_the_builtin_id_table`]
/// requires the artefact to *still* disagree in exactly these places, so a reissued artefact that
/// fixed one turns the case red and the row goes.
const CELL_STYLE_ARTEFACT_DEFECTS: &[(&str, &str, &str)] = &[
    (
        "heading1",
        "builtinId=\"17\"",
        "Annex G.2 gives `Heading 1` builtinId 16 and `Heading 2` builtinId 17, and the artefact's \
         own `<heading2>` also carries 17. Two styles cannot share a builtinId when §18.8.7 says \
         the builtinId determines the style, so 17 is a typo for 16 and 16 appears nowhere.",
    ),
    (
        "accent3",
        "no builtinId attribute at all",
        "Annex G.2 gives `Accent3` builtinId 37. The element carries the style but not the number, \
         so the artefact maps nothing for it.",
    ),
    (
        "normal",
        "a styleSheet whose named style is `Percent`, not `Normal`",
        "`<normal builtinId=\"0\">` holds the same styleSheet as `<percent builtinId=\"5\">` — a \
         copy in the artefact, not a statement about builtinId 0. Annex G.2's `Normal` at 0 is what \
         every producer in tests/fixtures/ writes and what this crate authors.",
    ),
];

/// **`presetCellStyles.xml` cannot settle the `builtinId` table, and the reason is measured.**
///
/// This is the negative half of MJXOFF-250 item 2, and it is the interesting half. Item 1's result
/// was that ECMA's published markup **confirmed** a hand-maintained table exactly. The reflex after
/// that is to treat every artefact the same way, and the reflex is wrong: the artefact beside it,
/// in the same directory, disagrees with the standard's own prose in three places — and in all
/// three the prose is right and the artefact is defective.
///
/// So the verdict *"not derivable"* is not left as a sentence in this file's header. It is asserted:
/// the disagreements must still be exactly [`CELL_STYLE_ARTEFACT_DEFECTS`], in both directions.
/// A fourth would mean the artefact is worse than recorded; a third that healed would mean ECMA has
/// reissued it and the question is worth reopening.
#[test]
fn ecmas_preset_cell_styles_cannot_settle_the_builtin_id_table() {
    let Some(source) = published(CELL_STYLES, "the published preset cell styles") else {
        return;
    };

    // Annex G.2, as this project states it — read out of `builtin_cell_style_name`'s own arms so
    // the comparison is against the table the library actually answers with.
    let table = read("crates/mjx-sml/src/styles/named_styles.rs");
    let start = table
        .find("pub const fn builtin_cell_style_name")
        .expect("mjx-sml declares `builtin_cell_style_name`");
    let end = start
        + table[start..]
            .find("\n}")
            .expect("`builtin_cell_style_name` closes");
    let mut annex: BTreeMap<String, u32> = BTreeMap::new();
    for line in table[start..end].lines() {
        let line = line.trim();
        let Some((id, rest)) = line.split_once(" => Fixed(\"") else {
            continue;
        };
        let Some((name, _)) = rest.split_once('"') else {
            continue;
        };
        if let Ok(id) = id.parse::<u32>() {
            annex.insert(name.to_owned(), id);
        }
    }
    assert!(
        annex.len() >= 40,
        "only {} Annex G.2 entries were read out of `builtin_cell_style_name` — the arm scan has \
         stopped matching",
        annex.len()
    );

    // The artefact: each top-level child is one preset, named by its tag, carrying its `builtinId`
    // and one `styleSheet` whose non-`Normal` `cellStyle` is the style's invariant name.
    let mut disagreements: BTreeMap<String, String> = BTreeMap::new();
    let mut examined = 0usize;
    for (slug, block) in top_level_blocks(&source) {
        examined += 1;
        let declared = attribute(&block, "builtinId").and_then(|v| v.parse::<u32>().ok());
        // `xfId="1"` is the style itself; `xfId="0"` is the `Normal` record every styleSheet also
        // carries. Excel appends " 2" to a name already in use, and two of the artefact's do carry
        // it — that is cosmetic, so it is trimmed rather than reported.
        let named = block
            .match_indices("<cellStyle ")
            .map(|(at, _)| &block[at..])
            .find(|tail| attribute(tail, "xfId").as_deref() == Some("1"))
            .and_then(|tail| attribute(tail, "name"));

        let Some(named) = named else { continue };
        // The suffix is only Excel's if trimming it lands on a name Annex G.2 has and the untrimmed
        // one is not: `60% - Accent1 2` is `60% - Accent1` renamed, while `Heading 2` is a style.
        let named = match named.strip_suffix(" 2") {
            Some(trimmed) if !annex.contains_key(&named) && annex.contains_key(trimmed) => {
                trimmed.to_owned()
            }
            _ => named,
        };
        // `RowLevel_n` / `ColLevel_n` are not fixed names in Annex G.2 — builtinId 1 and 2 are
        // prefixes with the outline level carried separately in `@iLevel` — so the artefact's
        // seven-of-each is agreement, not disagreement, and this comparison has nothing to say.
        if named.starts_with("RowLevel_") || named.starts_with("ColLevel_") {
            continue;
        }
        let expected = annex.get(&named).copied();
        match (declared, expected) {
            (Some(declared), Some(expected)) if declared == expected => {}
            (declared, expected) => {
                disagreements.insert(
                    slug,
                    format!(
                        "holds `{named}`; the artefact says builtinId {} and Annex G.2 says {}",
                        declared.map_or_else(|| "none".to_owned(), |id| id.to_string()),
                        expected.map_or_else(|| "none".to_owned(), |id| id.to_string())
                    ),
                );
            }
        }
    }
    assert!(
        examined > 40,
        "only {examined} top-level element(s) were read out of {CELL_STYLES} — the block scan has \
         stopped matching, and a run that finds nothing reports perfect agreement"
    );

    let registered: BTreeSet<&str> = CELL_STYLE_ARTEFACT_DEFECTS
        .iter()
        .map(|(slug, _, _)| *slug)
        .collect();
    let found: BTreeSet<&str> = disagreements.keys().map(String::as_str).collect();

    let unregistered: Vec<&&str> = found.difference(&registered).collect();
    assert!(
        unregistered.is_empty(),
        "{CELL_STYLES} disagrees with Annex G.2 somewhere this file does not record: {:?}\n\nWhat \
         each one says:\n  {}\n\nThat is either a defect in the artefact nobody has written down, \
         or — the possibility worth checking first — a defect in \
         `mjx_sml::styles::builtin_cell_style_name`. MJXOFF-246 is the reason to check that way \
         round: there, the standard's published data was right and this project's inference was \
         wrong.",
        unregistered,
        disagreements
            .iter()
            .map(|(slug, why)| format!("{slug} {why}"))
            .collect::<Vec<String>>()
            .join("\n  ")
    );

    let healed: Vec<&&str> = registered.difference(&found).collect();
    assert!(
        healed.is_empty(),
        "{:?} no longer disagree(s) with Annex G.2. ECMA has reissued {CELL_STYLES}, so delete the \
         row from `CELL_STYLE_ARTEFACT_DEFECTS` — and if the register empties, the artefact has \
         become an authority for this table after all and this file's header should say so.",
        healed
    );

    println!(
        "presetCellStyles.xml: {examined} presets examined against {} Annex G.2 entries; it \
         disagrees in exactly the {} recorded place(s): {:?}",
        annex.len(),
        found.len(),
        disagreements
    );
}

/// **Neither published styles artefact ever writes a logical edge border.**
///
/// The item-3 entry for `crates/mjx-sml/src/styles/borders.rs` says `x:start` and `x:end` are read
/// by analogy with WordprocessingML because §18.8 documents neither. The natural next question is
/// whether ECMA's published SpreadsheetML settles it, and the answer is no — measured here rather
/// than assumed, because *"the artefact does not help"* is exactly the kind of claim that is true
/// when written and quietly false after a reissue.
///
/// The positive half is what stops this passing vacuously: both artefacts must be full of the
/// **physical** edges, so a run that finds no logical edge because it found no borders at all fails.
#[test]
fn ecmas_published_markup_never_writes_a_logical_edge_border() {
    const TABLE_STYLES: &str = "OfficeOpenXML-SpreadsheetMLStyles/presetTableStyles.xml";
    for (relative, what) in [
        (CELL_STYLES, "the published preset cell styles"),
        (TABLE_STYLES, "the published preset table styles"),
    ] {
        let Some(source) = published(relative, what) else {
            return;
        };
        let count = |element: &str| {
            source
                .match_indices(&format!("<{element}"))
                .filter(|(at, _)| {
                    let after = at + 1 + element.len();
                    source[after..]
                        .chars()
                        .next()
                        .is_some_and(|c| c == ' ' || c == '>' || c == '/')
                })
                .count()
        };
        let physical = count("left") + count("right");
        assert!(
            physical > 50,
            "{relative} holds only {physical} `<left>`/`<right>` border element(s) — the element \
             scan has stopped matching, so the negative below is measuring nothing"
        );
        let logical = count("start") + count("end");
        assert_eq!(
            logical, 0,
            "{relative} writes {logical} `<start>`/`<end>` border element(s). It did not when \
             MJXOFF-250 was closed, and if it does now then ECMA's own published SpreadsheetML \
             *can* settle what those elements mean — which is the one thing this file's item-3 \
             entry for `crates/mjx-sml/src/styles/borders.rs` says nothing can."
        );
        println!(
            "{relative}: {physical} physical edge border element(s), {logical} logical — the \
             artefact cannot settle `x:start`/`x:end`"
        );
    }
}

// ===============================================================================================
// Small readers
// ===============================================================================================

/// The top-level children of a published artefact, as `(tag, the whole block)` pairs.
fn top_level_blocks(source: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let Some(root_open) = source.find('<').and_then(|_| {
        source
            .match_indices('<')
            .find(|(at, _)| {
                source[at + 1..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
            })
            .map(|(at, _)| at)
    }) else {
        return blocks;
    };
    let Some(root_end) = source[root_open..].find('>').map(|at| root_open + at + 1) else {
        return blocks;
    };

    let mut rest = &source[root_end..];
    let mut consumed = root_end;
    while let Some(at) = rest.find('<') {
        let tail = &rest[at + 1..];
        let Some(name) = tail
            .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .next()
            .filter(|name| !name.is_empty() && name.starts_with(|c: char| c.is_ascii_alphabetic()))
        else {
            rest = &rest[at + 1..];
            consumed += at + 1;
            continue;
        };
        let name = name.to_owned();
        let close = format!("</{name}>");
        let Some(end) = tail.find(&close).map(|to| to + close.len()) else {
            break;
        };
        let block = &source[consumed + at..consumed + at + 1 + end];
        blocks.push((name, block.to_owned()));
        rest = &rest[at + 1 + end..];
        consumed += at + 1 + end;
    }
    blocks
}

/// The value of `name` on the first start tag in `source`.
fn attribute(source: &str, name: &str) -> Option<String> {
    let tag_end = source.find('>')?;
    let tag = &source[..tag_end];
    let needle = format!("{name}=\"");
    let at = tag.find(&needle)? + needle.len();
    let to = tag[at..].find('"')? + at;
    Some(tag[at..to].to_owned())
}
