//! **Every owned checklist row names the corporate-fixture element that exercises it** (MJXOFF-300).
//!
//! RC03 commits one corporate-shaped fixture per format. A fixture is only worth committing if a
//! reader can say *which feature each part of it is there to exercise*, so each one carries a README
//! with a coverage table, and this gate holds that table against
//! `docs/client-platform/data/features.json` in both directions.
//!
//! # The form a README must use
//!
//! `tests/render/RC03-corporate/<format>/README.md`, carrying a section that begins with the
//! heading [`COVERAGE_HEADING`] and then a Markdown table of exactly two columns:
//!
//! ```text
//! ## Checklist coverage
//!
//! | features.json row | fixture element |
//! | --- | --- |
//! | pptx-line-breaks | the title's soft line break (`a:br`) |
//! ```
//!
//! The left column is a `features.json` `id`; the right says what in the fixture exercises it, in
//! words, and may not be empty. A table is parsed, never trusted: an id that names no row fails, and
//! a row that no table names fails the other way.
//!
//! # The population this gate sweeps
//!
//! MJXOFF-300 asks for *"every `features.json` row whose owner is a Wave 0–3 ticket"*. The epic's
//! waves are Wave 0 = RC00, RC01, RC47, RC02, RC03, RC04, RC05, RC06, RC07; Wave 1 = RC08–RC18;
//! Wave 2 = RC20–RC37; Wave 3 = RC38–RC45; Wave 4 = RC46 alone. **So "Waves 0–3" is every RC ticket
//! except RC46**, and that is the one subtraction this gate makes: [`WAVE_FOUR`] names it, and
//! every other roster-owned, in-scope row is demanded.
//!
//! Until the implementation half of MJXOFF-300 landed, this file swept every roster-owned row
//! instead, on the ground that no wave was written down anywhere in the repository. That was the
//! safe direction to be wrong in rather than the right population: RC46 is the epic's *final*
//! sweep, and demanding a corporate fixture element for a row whose own ticket has not been
//! scoped yet would have forced an excuse written before anyone could know whether one was true.
//!
//! A row the waves demand and no corporate file can carry is not silently dropped: it goes in
//! [`NOT_IN_THE_CORPORATE_FIXTURES`] with the reason, which is a statement rather than a
//! suppression.

#[path = "../src/json.rs"]
mod json;
#[path = "../src/ticket_roster.rs"]
mod ticket_roster;

use json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use ticket_roster::{closed_reason, is_roster_ticket, TICKET_ROSTER};

/// The heading that opens a README's coverage table.
const COVERAGE_HEADING: &str = "## Checklist coverage";

/// The three corporate fixtures, as (format, fixture file, README directory).
const CORPORATE_FIXTURES: [(&str, &str, &str); 3] = [
    ("pptx", "corporate.pptx", "tests/render/RC03-corporate/pptx"),
    ("xlsx", "corporate.xlsx", "tests/render/RC03-corporate/xlsx"),
    ("docx", "corporate.docx", "tests/render/RC03-corporate/docx"),
];

/// A row the corporate fixtures deliberately do not exercise, and why.
///
/// A row here is a **statement**, not a suppression: it does not turn the sweep off, it answers the
/// question the sweep raised. Writing one means having decided that a corporate-shaped file has no
/// natural place for the feature — not that covering it was inconvenient.
struct Excused {
    /// The `features.json` `id`.
    row: &'static str,
    /// Why no corporate fixture carries an element for it.
    reason: &'static str,
}

/// Every owned row no corporate fixture exercises, each with the reason it does not.
///
/// The rows fall into three kinds, and the reasons say which:
///
/// 1. **Not an element of a document at all** — an engine's capability, an exporter's behaviour, a
///    budget, a ledger. No fixture can carry one, because there is nothing in a file to carry.
/// 2. **A feature a corporate file would not have** — ink, handout pages, WordArt, chartsheets,
///    Arabic justification in an English report.
/// 3. **A feature this fixture does not state, whose owning ticket carries its own specimen.** These
///    are written plainly rather than dressed up: a corporate document *could* underline a word, and
///    this one does not. What makes the row safe to leave here is that the ticket which implements
///    it builds the case that proves it renders; what a corporate fixture adds is breadth, not that
///    proof.
const NOT_IN_THE_CORPORATE_FIXTURES: &[Excused] = &[
    // ---- PowerPoint ----
    Excused { row: "pptx-autofit", reason: "every text body here fits its shape; autofit is what happens when one does not, which RC37 states with a shape sized to force it" },
    Excused { row: "pptx-baseline-caps-spacing-kerning", reason: "typographic tuning a corporate template does not state; RC37 owns the run-property specimen" },
    Excused { row: "pptx-bullet-colour", reason: "the Wingdings bullets here take their colour from the run, which is the default; RC16 owns the specimen that states `a:buClr`" },
    Excused { row: "pptx-chart-label-text", reason: "the chart states no data labels: a bar chart on four quarters is read from its axis, and RC26 owns the labelled specimen" },
    Excused { row: "pptx-comment-markers", reason: "a comment is a reviewer's annotation on a deck, not part of the deck somebody sends" },
    Excused { row: "pptx-custom-dash", reason: "the connector is solid; a custom dash pattern is a line *style* RC14 varies in its own specimen" },
    Excused { row: "pptx-east-asian-vertical-text", reason: "an English deck has no vertical East Asian text; RC39 owns the script specimens" },
    Excused { row: "pptx-embedded-fonts", reason: "embedding a font is a distribution choice about a package, not an element on a slide; RC25 owns it" },
    Excused { row: "pptx-group-fill", reason: "no shape here is grouped, so no shape inherits a group's fill; RC35 owns the grouped specimen" },
    Excused { row: "pptx-handout-pages", reason: "a handout master is a printing surface, and this deck is one slide" },
    Excused { row: "pptx-hidden-shapes-and-slides", reason: "a hidden shape is one somebody meant not to send; a corporate deck's every shape is meant to be seen" },
    Excused { row: "pptx-hyperlink-colour", reason: "no run here is a hyperlink; RC16 owns the specimen where `hlink` resolves from the theme" },
    Excused { row: "pptx-ink", reason: "ink is a stylus annotation, not something a deck is authored with" },
    Excused { row: "pptx-justify-distribute", reason: "the title and the list are left-aligned, as a corporate template sets them; RC37 varies the alignment" },
    Excused { row: "pptx-legacy-vml", reason: "nothing in this deck is legacy VML: it is authored by this workspace's writers, which emit DrawingML throughout" },
    Excused { row: "pptx-media-poster-frames", reason: "a poster frame belongs to embedded video, which a deck sent by e-mail does not carry" },
    Excused { row: "pptx-non-latin-autonumber", reason: "the list is bulleted rather than numbered, and its bullets are Latin-script; RC30 owns the autonumber schemes" },
    Excused { row: "pptx-notes-page", reason: "a notes page is a second surface behind the slide; this deck states none" },
    Excused { row: "pptx-ole-snapshot", reason: "an embedded OLE object is another application's document inside this one; RC45 owns it" },
    Excused { row: "pptx-picture-bullets", reason: "the bullets here are characters from Wingdings, not pictures; RC30 owns the picture-bullet specimen" },
    Excused { row: "pptx-picture-fill-on-shape", reason: "the two pictures are `p:pic` shapes rather than a picture *fill* on an autoshape, which is a different element RC12 states" },
    Excused { row: "pptx-preset-shadow", reason: "no shape here carries an effect list; RC35 owns the specimen that states one" },
    Excused { row: "pptx-script-font-slots", reason: "one script, one font slot; RC37 owns the specimen that exercises the East Asian and complex-script slots" },
    Excused { row: "pptx-shape-style-font-colour", reason: "no shape here names a `p:style`, so no font colour resolves through one; RC16 owns that ladder" },
    Excused { row: "pptx-svg-pictures", reason: "the pictures are PNG; an SVG picture is a different part type RC44 owns" },
    Excused { row: "pptx-tab-alignment", reason: "no paragraph here states a tab stop; RC37 owns the specimen" },
    Excused { row: "pptx-text-effects", reason: "glow, reflection and shadow on text belong to a poster rather than a review deck" },
    Excused { row: "pptx-text-rotation", reason: "every text body here is horizontal; RC37 owns the rotated specimen" },
    Excused { row: "pptx-three-dimensional", reason: "a 3-D bevel or camera is a decorative extreme; RC45 owns it" },
    Excused { row: "pptx-wordart-warp", reason: "warped text is WordArt, which a corporate deck does not use" },
    // ---- Excel ----
    Excused { row: "xlsx-autofilter-buttons", reason: "the table states no auto-filter, so there is no dropdown button to draw; RC31 owns the furniture" },
    Excused { row: "xlsx-chartsheets", reason: "a chartsheet is a sheet that is only a chart; this workbook's chart sits on the worksheet, which is where a report puts it" },
    Excused { row: "xlsx-comment-indicators", reason: "a cell comment is a reviewer's annotation rather than the report's content" },
    Excused { row: "xlsx-hyperlink-cells", reason: "no cell here links out; RC18 owns the specimen where a hyperlink's colour and underline resolve" },
    Excused { row: "xlsx-ink", reason: "ink is a stylus annotation, not something a workbook is authored with" },
    Excused { row: "xlsx-legacy-drawings", reason: "the drawing here is DrawingML, which is what this workspace's writers emit; a legacy `xl/drawings/vmlDrawingN.vml` arrives only from Office and RC44 owns it" },
    Excused { row: "xlsx-outline-headers-divider", reason: "the sheet states no outline levels, so there is no grouping divider to draw; RC31 owns it" },
    Excused { row: "xlsx-pattern-fills", reason: "the fills here are solid and theme-resolved; a hatch is a pattern RC34 states in its own specimen" },
    Excused { row: "xlsx-printed-pages", reason: "pagination for print is a property of a print run rather than an element in the sheet; RC36 owns it" },
    Excused { row: "xlsx-row-height-recompute", reason: "recomputing a row's height is what the box model does when content changes, not something a committed file states; RC34 owns it" },
    Excused { row: "xlsx-sparklines", reason: "a sparkline is an `x14:` extension this workspace preserves and does not author; RC43 owns it" },
    Excused { row: "xlsx-system-colours", reason: "every colour here is a theme slot or an explicit value; a system colour resolves from the host's palette, which RC10 states" },
    // ---- Word ----
    Excused { row: "docx-automatic-colour", reason: "the styles here state their colours; `w:val=\"auto\"` resolves against the page behind it, which RC09 gives something to resolve against" },
    Excused { row: "docx-caps-and-small-caps", reason: "no run here is capitalised by a property; RC29 owns the run-property specimen" },
    Excused { row: "docx-chart-paint", reason: "the workbook and the deck carry the charts this ticket commits; a chart in Word is the same `c:chartSpace` reached a third way, and RC23 owns that route" },
    Excused { row: "docx-column-separators", reason: "the section is one column, so there is no separator rule to draw; RC08 owns the multi-column specimen" },
    Excused { row: "docx-comment-ranges", reason: "this document carries tracked changes rather than comments: both are review markup, and RC32 owns the commented specimen" },
    Excused { row: "docx-compound-borders", reason: "the page border is single; a compound border is a border *style* RC14 varies" },
    Excused { row: "docx-east-asian-layout", reason: "an English report states no East Asian layout; RC39 owns the script specimens" },
    Excused { row: "docx-field-results", reason: "the header states its text directly, which is what a corporate header does; RC29 owns the specimen where a field's cached result is what shows" },
    Excused { row: "docx-form-fields-and-content-controls", reason: "a form field belongs to a form somebody fills in, not to a report somebody reads" },
    Excused { row: "docx-frames-and-drop-caps", reason: "a drop cap is a magazine device; RC38 owns the framed specimen" },
    Excused { row: "docx-gutter-valign-direction", reason: "a binding gutter and a vertical alignment belong to a printed booklet; this section states margins and a page border" },
    Excused { row: "docx-hidden-text", reason: "hidden text is an editing artefact; a document sent to a colleague carries none" },
    Excused { row: "docx-hyphenation", reason: "hyphenation is a setting this document does not turn on; RC39 owns the specimen" },
    Excused { row: "docx-incremental-reflow", reason: "reflow is what the box model does across an edit, not an element a committed file can carry; RC40 owns it" },
    Excused { row: "docx-ink", reason: "ink is a stylus annotation, not something a document is authored with" },
    Excused { row: "docx-inline-and-cell-charts", reason: "as `docx-chart-paint`: the chart this ticket commits sits in the workbook and the deck" },
    Excused { row: "docx-kashida-and-distribute", reason: "Arabic justification has no place in an English report; RC39 owns it" },
    Excused { row: "docx-page-background", reason: "this document states a page border rather than a page background; RC22 owns the backgrounded specimen" },
    Excused { row: "docx-paragraph-borders", reason: "the borders here are the page's and the table's; RC08 owns the paragraph-border specimen" },
    Excused { row: "docx-script-font-slots", reason: "one script, one font slot; RC29 owns the specimen that exercises the others" },
    Excused { row: "docx-smartart-and-ole", reason: "the deck carries the SmartArt this ticket commits; a Word diagram is the same `dgm:` parts reached a second way, and RC45 owns OLE" },
    Excused { row: "docx-spacing-scale-kerning", reason: "character spacing and scaling are typographic tuning a corporate template does not state; RC29 owns the specimen" },
    Excused { row: "docx-text-effects", reason: "glow and shadow on body text belong to a poster rather than a report" },
    Excused { row: "docx-vertical-alignment-and-position", reason: "the superscripts in this document belong to its equation, which carries its own; RC13 owns the run-level specimen" },
    // ---- Shared ----
    Excused { row: "shared-chart-data-tables", reason: "a data table under a chart is a chart *feature* RC42 states; the charts here are plain" },
    Excused { row: "shared-chart-numbers-and-text", reason: "how a chart formats its numbers and lays out its text is the chart engine's behaviour; RC26 proves it against a chart built for it" },
    Excused { row: "shared-chart-series-formatting", reason: "both charts here leave their series unformatted so the theme's accents show, which is the point of `pptx-theme-schemes`; RC06 states an explicit `spPr`" },
    Excused { row: "shared-chart-three-dimensional", reason: "a 3-D chart is a chart kind RC42 owns; these are two-dimensional bars" },
    Excused { row: "shared-chartex", reason: "`chartex` is a separate part type Office writes for newer chart kinds; nothing here authors one" },
    Excused { row: "shared-colour-glyphs-in-pdf", reason: "how a colour glyph survives PDF export is an exporter's behaviour, not an element a document carries" },
    Excused { row: "shared-drawingml-paint-once", reason: "that DrawingML is painted by one implementation for all three formats is a property of the code; no fixture can carry it, and the four-painter comparison is where it is proved" },
    Excused { row: "shared-drawingml-shape-layout", reason: "where DrawingML shape layout *lives* — one engine the three formats share — is a property of the crate graph rather than of any element in a file; RC27 owns it" },
    Excused { row: "shared-font-coverage", reason: "which glyphs a face covers is a property of the font, not of the document that names it; RC25 owns the coverage report" },
    Excused { row: "shared-furniture-layer", reason: "the furniture layer is where selection, gridlines and comment markers are drawn *over* a page; it is chrome around a document rather than content in one" },
    Excused { row: "shared-ledger-truth", reason: "that the parity ledger says only what the suites check is a property of the ledger generator; RC02 owns it and no fixture can carry it" },
    Excused { row: "shared-math-typesetter", reason: "this document carries an `m:oMath` and RC41 carries the typesetter that lays one out; the row is about the engine, not the element" },
    Excused { row: "shared-pdf-missing-face", reason: "what a PDF export does when a face is unavailable is an exporter's behaviour; RC02 proves it by withholding the face" },
    Excused { row: "shared-picture-adjustment-parity", reason: "brightness, contrast and recolour on a picture are adjustments this fixture's pictures do not state; RC11 owns the parity case" },
    Excused { row: "shared-smartart-layout-algorithms", reason: "running a diagram's layout algorithm is what RC42 builds; this fixture carries the *cached* drawing instead, which is what a file actually holds" },
    Excused { row: "shared-svg-and-metafiles", reason: "an SVG or a metafile is a picture format none of these fixtures embeds; RC44 owns them" },
    Excused { row: "shared-svg-selectable-text", reason: "whether exported SVG carries selectable text is an exporter's behaviour rather than an element in a document" },
    Excused { row: "shared-text-engine-breadth", reason: "the text engine's breadth — bidi, shaping, script itemisation — is measured against specimens chosen to stress it, not against a corporate page of English" },
    Excused { row: "shared-viewer-budgets", reason: "a viewport's byte ceilings and frame budget are the viewer's behaviour under load; no committed file can state one" },
];

/// RC46's ticket: the epic's Wave 4, and the one wave this gate does **not** demand coverage of.
///
/// Every other roster ticket is Wave 0–3. RC46 is the final sweep — the rows it owns are the ones
/// no earlier ticket claimed — so a corporate fixture element demanded for one of them would be an
/// element authored for work nobody has scoped. See this module's own documentation.
const WAVE_FOUR: &str = "MJXOFF-338";

/// A floor that says a README's table is still being parsed, not the exact size of any of them.
const MINIMUM_COVERED_ROWS_PER_FIXTURE: usize = 5;

/// A floor that says the checklist reader is still matching rows.
const MINIMUM_CHECKLIST_ROWS: usize = 100;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits one level below the repository root")
        .to_path_buf()
}

fn checklist() -> Value {
    let path = repository_root().join("docs/client-platform/data/features.json");
    let text = std::fs::read_to_string(&path).expect("the feature checklist is committed");
    json::parse(&text).expect("the feature checklist is valid JSON")
}

fn text<'a>(row: &'a Value, key: &str) -> &'a str {
    row.get(key).and_then(Value::string).unwrap_or("").trim()
}

/// Every checklist row, as (id, format), for the rows this gate demands coverage of.
///
/// A row is demanded when it has an owning ticket on the epic's roster and is not out of scope. A
/// closed owner is not demanded: its work is merged and the checklist gate already refuses it.
fn demanded_rows() -> BTreeMap<String, String> {
    let checklist = checklist();
    let rows = checklist
        .get("features")
        .and_then(Value::array)
        .expect("the checklist has a `features` array");
    assert!(
        rows.len() >= MINIMUM_CHECKLIST_ROWS,
        "only {} checklist row(s) were read; the reader has stopped matching",
        rows.len()
    );

    let mut demanded = BTreeMap::new();
    for row in rows {
        let (id, owner, format) = (text(row, "id"), text(row, "owner"), text(row, "format"));
        if id.is_empty() {
            continue;
        }
        if row.get("out_of_scope").is_some() || owner.is_empty() {
            continue;
        }
        if closed_reason(owner).is_some() || !is_roster_ticket(owner) {
            continue;
        }
        // Wave 4 is out of this gate's population; every other roster ticket is Wave 0-3.
        if owner == WAVE_FOUR {
            continue;
        }
        demanded.insert(id.to_owned(), format.to_owned());
    }
    assert!(
        !demanded.is_empty(),
        "no checklist row is owned by a roster ticket; this gate's population reader has broken"
    );
    demanded
}

/// Every id in the whole checklist, so an unknown id in a README can be told from an excused one.
fn every_row_id() -> BTreeSet<String> {
    let checklist = checklist();
    let rows = checklist
        .get("features")
        .and_then(Value::array)
        .expect("the checklist has a `features` array");
    rows.iter()
        .map(|row| text(row, "id").to_owned())
        .filter(|id| !id.is_empty())
        .collect()
}

/// One README's coverage table, as (row id, the element that exercises it).
///
/// # Panics
/// If the README is missing, carries no coverage heading, or its table is malformed — each of which
/// is a fixture that cannot say what it is for.
fn coverage_table(directory: &str) -> Vec<(String, String)> {
    let path = repository_root().join(directory).join("README.md");
    let markdown = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}. RC03 commits one README per corporate fixture, carrying the `{}` table \
             this gate reads.",
            path.display(),
            COVERAGE_HEADING
        )
    });

    let (_, after) = markdown.split_once(COVERAGE_HEADING).unwrap_or_else(|| {
        panic!(
            "{} carries no `{COVERAGE_HEADING}` heading, so it does not say which checklist rows \
             the fixture exercises",
            path.display()
        )
    });

    let mut covered = Vec::new();
    for line in after.lines() {
        let line = line.trim();
        if line.starts_with("##") {
            break; // The next section; the table has ended.
        }
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        if cells.len() != 2 {
            panic!(
                "{}: the coverage table must have exactly two columns, and this row has {}: {line}",
                path.display(),
                cells.len()
            );
        }
        let (id, element) = (cells[0], cells[1]);
        // The header row and the `| --- | --- |` separator carry no id.
        if id.eq_ignore_ascii_case("features.json row") || id.chars().all(|c| c == '-' || c == ':')
        {
            continue;
        }
        assert!(
            !element.is_empty(),
            "{}: row `{id}` names no fixture element. A coverage claim with nothing beside it is \
             the suppression this table exists not to be.",
            path.display()
        );
        covered.push((id.to_owned(), element.to_owned()));
    }
    covered
}

/// Every README's table parses, names only real rows, and names each of them once.
#[test]
fn every_readme_covers_real_checklist_rows() {
    let known = every_row_id();
    for (format, fixture, directory) in CORPORATE_FIXTURES {
        let covered = coverage_table(directory);
        assert!(
            covered.len() >= MINIMUM_COVERED_ROWS_PER_FIXTURE,
            "{fixture}'s README claims {} covered row(s); a table this short means the parser has \
             stopped matching or the fixture is not corporate-shaped",
            covered.len()
        );

        let mut seen = BTreeSet::new();
        for (id, _) in &covered {
            assert!(
                known.contains(id),
                "{fixture}'s README claims checklist row `{id}`, which is in no \
                 `docs/client-platform/data/features.json` row. A coverage claim against a row that \
                 does not exist is a claim nothing can check."
            );
            assert!(
                seen.insert(id.clone()),
                "{fixture}'s README claims row `{id}` twice"
            );
        }
        assert!(
            !format.is_empty(),
            "every corporate fixture names the format it is for"
        );
    }
}

/// A README may only claim rows of its own format, or rows shared across all three.
#[test]
fn a_readme_claims_only_its_own_formats_rows() {
    let checklist = checklist();
    let rows = checklist
        .get("features")
        .and_then(Value::array)
        .expect("the checklist has a `features` array");
    let formats: BTreeMap<String, String> = rows
        .iter()
        .map(|row| (text(row, "id").to_owned(), text(row, "format").to_owned()))
        .collect();

    for (format, fixture, directory) in CORPORATE_FIXTURES {
        for (id, _) in coverage_table(directory) {
            let row_format = formats.get(&id).map(String::as_str).unwrap_or("");
            assert!(
                row_format == format || row_format == "shared",
                "{fixture}'s README claims row `{id}`, whose format is `{row_format}`. A \
                 `.{format}` fixture cannot exercise another format's feature."
            );
        }
    }
}

/// **Every owned, in-scope checklist row is either exercised by a corporate fixture or excused in
/// writing.**
///
/// This is the gate MJXOFF-300 is really asking for. See this module's own documentation for why the
/// population is every roster-owned row rather than a wave.
#[test]
fn every_owned_row_is_covered_or_excused() {
    let demanded = demanded_rows();
    let known = every_row_id();

    let mut covered: BTreeSet<String> = BTreeSet::new();
    for (_, _, directory) in CORPORATE_FIXTURES {
        for (id, _) in coverage_table(directory) {
            covered.insert(id);
        }
    }

    let excused: BTreeMap<&str, &str> = NOT_IN_THE_CORPORATE_FIXTURES
        .iter()
        .map(|entry| (entry.row, entry.reason))
        .collect();
    for (row, reason) in &excused {
        assert!(
            known.contains(*row),
            "`{row}` is excused from the corporate fixtures and is in no checklist row"
        );
        assert!(
            !reason.trim().is_empty(),
            "`{row}` is excused with no reason written"
        );
        assert!(
            !covered.contains(*row),
            "`{row}` is both claimed by a fixture README and excused from the fixtures. It is one \
             or the other."
        );
    }

    let missing: Vec<&String> = demanded
        .keys()
        .filter(|id| !covered.contains(*id) && !excused.contains_key(id.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "{} owned checklist row(s) are neither exercised by a corporate fixture nor excused from \
         them. Add the element to the fixture and a line to its README's `{COVERAGE_HEADING}` \
         table, or add the row to `NOT_IN_THE_CORPORATE_FIXTURES` with the reason a \
         corporate-shaped file has no place for it. The rows: {missing:?}",
        missing.len()
    );
}

/// The roster this gate reads its ownership from is the epic's own, and it is not empty.
///
/// Without this, a `TICKET_ROSTER` that had emptied would make `demanded_rows` empty and every
/// sweep above pass over nothing.
#[test]
fn the_ticket_roster_is_the_epics_roster() {
    assert!(
        TICKET_ROSTER.len() >= 40,
        "the epic's ticket roster holds {} pair(s); a roster this short means this gate's ownership \
         reader is looking at the wrong table",
        TICKET_ROSTER.len()
    );
    assert!(
        TICKET_ROSTER.iter().any(|(rc, _)| *rc == "RC03"),
        "RC03 is not on the roster this gate reads, so the ticket that commits the corporate \
         fixtures is not one it can recognise"
    );
}
