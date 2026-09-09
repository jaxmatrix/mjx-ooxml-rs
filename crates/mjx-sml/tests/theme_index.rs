//! What a SpreadsheetML `@theme` position means (MJXOFF-246).
//!
//! # The contradiction this suite closes
//!
//! `crates/mjx-sml/src/write/stylesheet.rs` authors font 0 — the font every cell that names no font
//! of its own draws with — as the theme's **first text colour**, which is what Excel's own font 0
//! states and what every third-party producer in `tests/fixtures/` copies. Until this suite,
//! `crates/mjx-sml/src/styles/palette.rs` resolved that same position to `lt1`, the theme's
//! **background**. The library therefore read the default font colour of every workbook it authored
//! as *white*, and a renderer built on `effective_cell_format` would have painted white text on a
//! white sheet. Nothing caught it: both candidate slots are defined in every theme, so the reference
//! gate resolved either way, schema validity passed, and the round trip passed.
//!
//! Two tests hold the two halves together, and they fail for different reasons:
//!
//! * [`the_default_font_this_crate_authors_resolves_to_a_dark_colour`] is the **agreement** gate. It
//!   runs the whole path — author a package, read `xl/styles.xml` back out of it, take font 0's
//!   `<color>` *as the file states it*, read `xl/theme/theme1.xml` back out of the same package, and
//!   resolve one against the other. Nothing in it re-types the number the writer used, so a writer
//!   and a resolver that disagree cannot both be satisfied.
//! * [`the_theme_index_mapping_is_the_one_ecmas_own_preset_styles_use`] is the **derivation** gate.
//!   It re-derives the mapping from data this project did not author and cannot edit, and is the
//!   reason the module documentation in `palette.rs` can state a conclusion rather than a preference.
//!
//! # Why the derivation gate is not circular, and what it reads
//!
//! ECMA publishes more than the prose. The Part 1 5th-edition package carries
//! `OfficeOpenXML-SpreadsheetMLStyles/`, and two of its three files are SpreadsheetML **styles**
//! markup written by the standard itself:
//!
//! | File | What it is |
//! |---|---|
//! | `presetCellStyles.xml` | the built-in cell styles — `Normal`, `Title`, `Heading 1`…`4`, `Check Cell`, `Accent1`…`Accent6` — one `styleSheet` each |
//! | `presetTableStyles.xml` | the built-in table styles, `TableStyleLight1`…`TableStyleDark11`, as `dxf` bands |
//!
//! Both spell colours as `@theme` positions, and both were authored to be **legible**: a style that
//! painted its text the colour of its own fill would be a style nobody could use. So the position
//! table is recoverable from them without asking Microsoft anything and without reading a file
//! Office wrote — resolve each font colour and the ground it sits on against a concrete colour
//! scheme, and require the two to differ.
//!
//! Under the mapping this crate now implements, the smallest separation ECMA's own data contains is
//! **39.7** of 255 (`Accent4`'s white text over its own accent lightened 40%), so *nothing* falls
//! under the floor. Under the mapping §20.1.6.2's *sequence* table would give, **144** pairs fall
//! under it and the worst are **exactly 0.0** — `Normal`'s font comes out white on a white sheet, and
//! a table style paints `theme="0"` text onto a `theme="0"` fill. The floor sits in the gap between
//! those two, so it is not a threshold tuned to pass: every value in a wide band decides the same
//! way, and both figures are the ones the assertion below prints when it is made to fail.
//!
//! # Where a literal position may still be written, and where it may not
//!
//! MJXOFF-246's defect was one decision stated twice in different words. So the standing line this
//! suite draws, applied across the crate and both bindings:
//!
//! * **A test may not assert a position as a check.** That is a second copy of a decision made in
//!   `theme_color_position`, sitting where nothing compares the two. Three did — a
//!   `highlight_from_theme` doctest and both bindings' surface-coverage tests, each pinning
//!   `Dark1 → 0` — and all three are now written as *properties*: the doctest reads the position
//!   back through [`theme_color_slot`], and the bindings assert that the twelve slots occupy the
//!   twelve positions exactly once each and that `Dark1`'s position is **not** its ordinal in the
//!   enumeration. That last one is the sharp claim, because projecting the ordinal is much the
//!   likeliest way for a binding to get this wrong and it passes every other slot.
//! * **A doctest may show a position as documentation.** `solid_from_theme`, `two_color_from_theme`,
//!   `spanning_the_range_from_theme` and `Color::from_theme_slot` all state an accent — `Some(4)`,
//!   `Some(5)` — and an accent's position is the same under both readings of the table, so nothing
//!   there was ever at risk. Showing the number is what makes those examples teach.
//!
//! The one restatement kept on purpose is `crates/mjx-sml/tests/style_resources.rs`'s
//! cross-vocabulary table, because it is literal *by design*: it holds on every machine, where this
//! suite's derivation skips without `References/`.
//!
//! # Why this is a *skipping* gate
//!
//! `References/` is git-ignored, by the same standing rule that makes `mjx-schema-gate` skip without
//! it. This suite follows that convention exactly: absent, it prints a notice and passes; with
//! `MJX_REQUIRE_SCHEMA=1` set — which is what CI sets — the absence is a failure instead.

use std::collections::BTreeSet;
use std::path::PathBuf;

use mjx_dml::{ResolvedColor, SchemeColors, Theme};
use mjx_ooxml_core::convert::FromXml;
use mjx_sml::styles::{resolve_color, theme_color_slot, IndexedColorPalette, StylesheetPart};
use mjx_sml::write::WorkbookPackage;
use mjx_sml::Color;
use mjx_xml::{Element, Event, Reader};

/// Perceived brightness of a resolved colour, `0.0` (black) to `255.0` (white).
///
/// Rec. 601 luma. The choice of coefficients is not load-bearing — every candidate separates the two
/// readings by more than an order of magnitude — but a weighted luma rather than a plain mean is
/// what makes "dark" mean what a reader means by it.
fn brightness(color: ResolvedColor) -> f64 {
    0.299 * f64::from(color.red) + 0.587 * f64::from(color.green) + 0.114 * f64::from(color.blue)
}

/// Reads one part out of a package by its literal name.
fn part_bytes(package: &mjx_opc::Package, name: &str) -> Vec<u8> {
    let part = mjx_opc::PartName::new(name).expect("a literal part name");
    package
        .part_bytes(&part)
        .unwrap_or_else(|| panic!("the authored package has {name}"))
        .to_vec()
}

/// The colour scheme of a theme part, as [`SchemeColors`].
fn scheme_of(theme_xml: &[u8]) -> SchemeColors {
    let document = mjx_xml::fidelity::parse(theme_xml).expect("the theme part parses");
    let theme = Theme::from_xml(&document.root, &document.interner).expect("the theme reads");
    let scheme = theme.color_scheme().expect("the theme has a colour scheme");
    SchemeColors::from_scheme(scheme, &document.interner)
}

/// The **agreement** gate: what the writer authors for font 0, resolved by the reader, is dark.
///
/// The colour is not re-typed here. It is read out of the `xl/styles.xml` the writer produced, and
/// the theme it resolves against is read out of the `xl/theme/theme1.xml` the same writer produced,
/// so this fails whenever the two halves of the crate mean different things by the same number —
/// which is the defect MJXOFF-246 records, and which nothing else in the workspace could see.
#[test]
fn the_default_font_this_crate_authors_resolves_to_a_dark_colour() {
    let mut workbook = WorkbookPackage::new().expect("a blank workbook");
    let package = workbook.to_package().expect("the package writes");

    let styles_bytes = part_bytes(&package, "/xl/styles.xml");
    let styles_document = mjx_xml::fidelity::parse(&styles_bytes).expect("the stylesheet parses");
    let stylesheet = StylesheetPart::read_part(&styles_document)
        .expect("the stylesheet reads")
        .expect("the part is a styleSheet");

    let font_zero = stylesheet
        .fonts()
        .expect("the skeleton writes a font table")
        .get(0)
        .expect("the skeleton writes font 0")
        .properties(&styles_document.interner);
    let color = font_zero.color.expect("font 0 states a colour");

    assert!(
        color.theme.is_some(),
        "font 0 must follow the document's theme rather than pin a colour into the file: {color:?}"
    );

    let scheme = scheme_of(&part_bytes(&package, "/xl/theme/theme1.xml"));
    let resolved = resolve_color(&color, &scheme, &IndexedColorPalette::default_palette())
        .expect("font 0's colour resolves against the theme the same package carries");
    let luma = brightness(resolved);

    assert!(
        luma < 96.0,
        "the default font colour of every workbook this crate authors resolves to {resolved:?} \
         (brightness {luma:.1}), which is not text anyone can read on a white sheet. The writer \
         authors font 0 as the theme's first *text* colour; if the resolver disagrees about which \
         scheme slot that position names, this is what the disagreement looks like. See MJXOFF-246 \
         and the module documentation of `mjx_sml::styles::palette`."
    );
}

// ---------------------------------------------------------------------------------------------
// The derivation gate
// ---------------------------------------------------------------------------------------------

/// One legibility constraint ECMA's own markup states: a font colour, and what it is drawn on.
#[derive(Debug)]
struct Legibility {
    /// Where in ECMA's markup this came from, for the failure message.
    source: String,
    /// The font's `<color>`.
    text: Color,
    /// The fill behind it, or `None` for "the sheet's own background", which is white.
    ground: Option<Color>,
}

/// `References/…/OfficeOpenXML-SpreadsheetMLStyles`, or `None` when the gate should skip.
///
/// Mirrors `mjx_schema_gate::harness`'s convention rather than inventing a second one: the directory
/// is located from an environment override or from the git-ignored `References/` tree at the
/// workspace root, **both** of its files must be present, and `MJX_REQUIRE_SCHEMA` turns an absence
/// into a failure rather than a skip.
///
/// # Panics
/// If `MJX_REQUIRE_SCHEMA` is set and the files are not there.
fn preset_styles_dir() -> Option<PathBuf> {
    const DEFAULT: &str = "ECMA-376-1_5th_edition_december_2016/OfficeOpenXML-SpreadsheetMLStyles";
    const FILES: [&str; 2] = ["presetCellStyles.xml", "presetTableStyles.xml"];

    let candidate = match std::env::var_os("MJX_PRESET_STYLES_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("References")
            .join(DEFAULT),
    };

    if FILES.iter().all(|file| candidate.join(file).is_file()) {
        return Some(candidate);
    }
    assert!(
        std::env::var_os("MJX_REQUIRE_SCHEMA").is_none(),
        "MJX_REQUIRE_SCHEMA is set but ECMA's preset styles (References/{DEFAULT}, or \
         MJX_PRESET_STYLES_DIR) could not be found at {}",
        candidate.display()
    );
    eprintln!(
        "skipping the theme-index derivation: ECMA's preset styles are not on this machine \
         (References/{DEFAULT}, or MJX_PRESET_STYLES_DIR)"
    );
    None
}

/// Reads a `<color>`-shaped element's attributes into a [`Color`].
///
/// Only the three spellings these two files use. An attribute that does not parse is left absent,
/// which makes the pair it belongs to unresolvable and therefore skipped, rather than silently
/// becoming a different colour.
fn color_of(element: &Element) -> Color {
    Color {
        theme: element.attr("theme").and_then(|value| value.parse().ok()),
        tint: element.attr("tint").and_then(|value| value.parse().ok()),
        rgb: element.attr("rgb").map(str::to_owned),
        ..Color::default()
    }
}

/// Strips a UTF-8 byte-order mark. ECMA writes one on both files.
fn strip_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes)
}

/// One start-shaped event, with whether it was `<x/>` rather than `<x>`.
///
/// `Reader` reports the two separately and an `Empty` produces no matching `End`, so a scanner that
/// tracks nesting has to know which it was handed. Returning it beside the element is what keeps
/// both scanners below reading as one pass rather than two interleaved ones.
enum Open {
    /// `<x>` or `<x/>`, with `true` for the self-closing spelling.
    Element(Element, bool),
    /// `</x>`, by local name.
    Close(String),
    /// Nothing more.
    Done,
}

/// The next start, end or end-of-input, with text discarded.
fn next_open(reader: &mut Reader<'_>, what: &str) -> Open {
    loop {
        match reader.read().unwrap_or_else(|e| panic!("{what}: {e}")) {
            Event::Start(element) => return Open::Element(element, false),
            Event::Empty(element) => return Open::Element(element, true),
            Event::End(name) => return Open::Close(name.local),
            Event::Text(_) => {}
            Event::Eof => return Open::Done,
        }
    }
}

/// The legibility constraints in `presetCellStyles.xml`.
///
/// Each entry is one built-in cell style as a whole `styleSheet`. Font 0 is always `Normal`'s and
/// every entry restates it; the style's own font is the one after it. A cell style paints on the
/// sheet unless it supplies a fill of its own, so a font with no solid fill beside it is constrained
/// against **white** — a fact about a spreadsheet's background, not about the table being derived.
fn cell_style_constraints(markup: &[u8]) -> Vec<Legibility> {
    let mut reader = Reader::new(strip_bom(markup));
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut entry = String::new();
    let mut in_fills = false;
    let mut in_solid_fill = false;
    let mut in_font = false;
    let mut font_color: Option<Color> = None;
    // The font's own index in the table, counted rather than inferred from how many colours were
    // collected: a font with no `<color>` contributes no constraint but still shifts the numbering,
    // and it is the *index* that says whether a font is `Normal`'s or the style's own.
    let mut font_index = 0usize;
    let mut fonts: Vec<(usize, Color)> = Vec::new();
    let mut solid: Option<Color> = None;

    loop {
        let (element, self_closing) = match next_open(&mut reader, "ECMA's preset cell styles") {
            Open::Element(element, self_closing) => (element, self_closing),
            Open::Close(local) => {
                depth -= 1;
                match local.as_str() {
                    "fills" => in_fills = false,
                    "patternFill" => in_solid_fill = false,
                    "font" => {
                        in_font = false;
                        fonts.extend(font_color.take().map(|color| (font_index, color)));
                        font_index += 1;
                    }
                    "styleSheet" => {
                        for (index, text) in fonts.drain(..) {
                            out.push(Legibility {
                                source: format!("presetCellStyles.xml {entry} font {index}"),
                                text,
                                // Font 0 is `Normal`'s and draws on the sheet; the style's own font
                                // draws on the style's own fill when it has one.
                                ground: if index == 0 { None } else { solid.clone() },
                            });
                        }
                        solid = None;
                        font_index = 0;
                    }
                    _ => {}
                }
                continue;
            }
            Open::Done => break,
        };

        depth += 1;
        match element.local() {
            // Depth 2 is the wrapper naming the built-in style, one per `styleSheet`.
            name if depth == 2 => entry = name.to_owned(),
            "fills" => in_fills = true,
            "patternFill" if in_fills => {
                in_solid_fill = element.attr("patternType") == Some("solid");
            }
            "fgColor" if in_solid_fill => solid = Some(color_of(&element)),
            "font" => {
                in_font = true;
                font_color = None;
            }
            "color" if in_font => font_color = Some(color_of(&element)),
            _ => {}
        }

        if self_closing {
            depth -= 1;
            match element.local() {
                "fills" => in_fills = false,
                "patternFill" => in_solid_fill = false,
                "font" => {
                    in_font = false;
                    fonts.extend(font_color.take().map(|color| (font_index, color)));
                    font_index += 1;
                }
                _ => {}
            }
        }
    }
    out
}

/// The legibility constraints in `presetTableStyles.xml`.
///
/// Only a `dxf` stating **both** a font colour and a fill is a constraint. A band that sets one
/// alone inherits the other from a different `dxf` in the same table style, so pairing it with the
/// sheet's background would assert something ECMA's markup does not say — and would, measurably,
/// produce sixty-eight false constraints.
fn table_style_constraints(markup: &[u8]) -> Vec<Legibility> {
    let mut reader = Reader::new(strip_bom(markup));
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut style = String::new();
    let mut in_dxf = false;
    let mut in_font = false;
    let mut in_pattern = false;
    let mut text: Option<Color> = None;
    let mut ground: Option<Color> = None;

    loop {
        let (element, self_closing) = match next_open(&mut reader, "ECMA's preset table styles") {
            Open::Element(element, self_closing) => (element, self_closing),
            Open::Close(local) => {
                depth -= 1;
                match local.as_str() {
                    "font" => in_font = false,
                    "patternFill" => in_pattern = false,
                    "dxf" => {
                        in_dxf = false;
                        if let (Some(text), Some(ground)) = (text.take(), ground.take()) {
                            out.push(Legibility {
                                source: format!("presetTableStyles.xml {style} dxf"),
                                text,
                                ground: Some(ground),
                            });
                        }
                    }
                    _ => {}
                }
                continue;
            }
            Open::Done => break,
        };

        depth += 1;
        match element.local() {
            name if depth == 2 => style = name.to_owned(),
            "dxf" => {
                in_dxf = true;
                text = None;
                ground = None;
            }
            "font" if in_dxf => in_font = true,
            "color" if in_font => text = Some(color_of(&element)),
            "patternFill" if in_dxf => in_pattern = true,
            // For a solid pattern the foreground is the fill. These presets state both and state
            // them identically, so preferring `fgColor` changes nothing here and is the reading
            // `sml.xsd` gives; the `bgColor` arm is what catches a band that writes only one.
            "fgColor" if in_pattern => ground = Some(color_of(&element)),
            "bgColor" if in_pattern && ground.is_none() => ground = Some(color_of(&element)),
            _ => {}
        }

        if self_closing {
            depth -= 1;
            match element.local() {
                "font" => in_font = false,
                "patternFill" => in_pattern = false,
                _ => {}
            }
        }
    }
    out
}

/// The **derivation** gate: this crate's position table is the one ECMA's own SpreadsheetML uses.
///
/// See the module documentation for what is read and why it is not circular. The claim is stated as
/// a legibility separation rather than as a position table, so this asserts the *property* the
/// mapping has to have and never restates the mapping itself — a table that had drifted in any of
/// the positions the two candidate readings disagree about reddens here.
#[test]
fn the_theme_index_mapping_is_the_one_ecmas_own_preset_styles_use() {
    let Some(dir) = preset_styles_dir() else {
        return;
    };
    let read = |name: &str| {
        let path = dir.join(name);
        std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    };

    let mut constraints = cell_style_constraints(&read("presetCellStyles.xml"));
    let from_cell_styles = constraints.len();
    constraints.extend(table_style_constraints(&read("presetTableStyles.xml")));
    let from_table_styles = constraints.len() - from_cell_styles;

    // A floor phrased as *the scanners are still matching*, never as an exact total: ECMA may
    // reissue the artefact, and a count here would be a ledger rather than a check. What has to hold
    // is that both files yielded constraints and that there are enough for a partial parse to show.
    assert!(
        from_cell_styles > 50 && from_table_styles > 50,
        "the preset-style scanners have stopped matching: {from_cell_styles} constraints from \
         presetCellStyles.xml and {from_table_styles} from presetTableStyles.xml"
    );

    // The scheme this library itself authors, so the derivation runs against a theme that is in the
    // repository rather than one quoted from a document.
    let scheme = scheme_of(mjx_dml::DEFAULT_THEME_XML.as_bytes());
    let palette = IndexedColorPalette::default_palette();

    /// Below this, a font and the ground behind it are not two colours a reader can tell apart.
    ///
    /// ECMA's own worst pair separates by 39.7, and the reading §20.1.6.2's sequence table would
    /// give produces pairs separating by 0.0. Any floor in that gap decides the same way, so this
    /// one is not tuned to a boundary.
    const LEGIBLE: f64 = 24.0;

    let mut illegible = Vec::new();
    let mut compared = 0usize;
    for constraint in &constraints {
        let Some(text) = resolve_color(&constraint.text, &scheme, &palette) else {
            continue;
        };
        let ground = match &constraint.ground {
            Some(fill) => match resolve_color(fill, &scheme, &palette) {
                Some(resolved) => brightness(resolved),
                None => continue,
            },
            // No fill: the sheet's own background, which is white in every spreadsheet.
            None => 255.0,
        };
        compared += 1;
        let separation = (brightness(text) - ground).abs();
        if separation < LEGIBLE {
            illegible.push(format!(
                "  {separation:6.1} — {} · text {:?} on {:?}",
                constraint.source, constraint.text, constraint.ground
            ));
        }
    }
    assert!(
        compared > 100,
        "the constraints were collected but almost none resolved ({compared} of {}), so this \
         compared nothing",
        constraints.len()
    );

    assert!(
        illegible.is_empty(),
        "{} of {compared} colour pairs ECMA's own preset styles author come out illegible under \
         this crate's `theme_color_slot`, which means the position table is wrong. ECMA authored \
         these styles to be readable; a mapping that paints their text the colour of their own \
         ground is not the mapping their markup was written against.\n{}\nSee MJXOFF-246 and the \
         module documentation of `mjx_sml::styles::palette`.",
        illegible.len(),
        illegible
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );

    // The derivation would be vacuous if nothing it read used the positions in dispute, so it says
    // so out loud: the two readings differ only on `0`..=`3`, and ECMA's markup has to name them for
    // any of the above to have decided anything.
    //
    // The set is asserted *exactly*, and the exactness is not brittle here: the archive these two
    // files come out of is pinned by SHA-256 in `.github/ecma-376-archives.sha256`, so the data
    // cannot change without someone editing that manifest. What the exact set records is worth
    // having on its own — **position 2 is missing**. ECMA's preset styles never name it, so the
    // evidence covers `lt1`/`dk1` directly, covers `dk2` through `Title` and the four headings, and
    // reaches `lt2` only by the symmetry of the swap. That is the honest extent of the derivation
    // and the one place it rests on an argument rather than on a measurement.
    let disputed: BTreeSet<u32> = constraints
        .iter()
        .flat_map(|c| [c.text.theme, c.ground.as_ref().and_then(|g| g.theme)])
        .flatten()
        .filter(|position| *position < 4)
        .collect();
    assert_eq!(
        disputed,
        [0, 1, 3].into_iter().collect::<BTreeSet<u32>>(),
        "ECMA's preset styles are what decide the disputed positions; if they stopped naming them, \
         this gate would be green for the wrong reason"
    );
    for position in &disputed {
        assert!(
            theme_color_slot(*position).is_some(),
            "position {position} is one ECMA's own styles use and must name a slot"
        );
    }
}
