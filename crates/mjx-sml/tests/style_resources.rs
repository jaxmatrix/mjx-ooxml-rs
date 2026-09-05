//! **MJXOFF-105's markup gate.** `styles.xml`'s resource tables: read, held in schema position,
//! written back byte for byte, and resolved to actual colours.
//!
//! # The fixture, and why it is authored the way it is
//!
//! Six Phase A children in a row shipped a test that could not fail, and twice the cause was a
//! fixture written in the order the writer emits. `tests/fixtures/style_resources.xlsx` is authored
//! against that, and specifically against the two mistakes this subject invites:
//!
//! * **Two byte-identical `<font>` entries** (indices 2 and 3). A model that deduplicated them
//!   would pass every "the fonts read back" assertion and fail
//!   [`font_indices_are_positions_and_appending_never_moves_them`]. A fixture whose fonts all
//!   differed could not tell the two apart.
//! * **A `<border>` exercising all nine edges**, `start` and `end` included. The ticket for this
//!   child said "the six edges plus diagonal"; `CT_Border` declares nine, and a fixture with only
//!   `left`/`right`/`top`/`bottom`/`diagonal` — which is what `sample.xlsx` writes — would let a
//!   seven-edge model pass.
//! * **A `dxf` with only a fill, a `dxf` with only a font, a `dxf` with all six members, and a
//!   `<dxf/>`.** *Inherited* and *default* are different states, and only the first and last
//!   together can say so.
//! * **An `indexedColors` block that differs from the default palette at exactly one row**
//!   (index 10). A replacement palette identical to the default cannot tell an implementation that
//!   honours the override from one that ignores it.
//! * **A `tableStyles`, a `cellXfs`, a `numFmts` and an `extLst`** — four of the six slots this
//!   child holds raw — plus a doubled space inside two start tags, a single-quoted attribute, an
//!   element written `<top …></top>` rather than self-closed, a comment between two slots, and an
//!   `ext` in a foreign namespace with its own prefix binding. None of those is reproduced by a
//!   rebuild from a decoded model, which is what makes the round-trip cases here a comparison
//!   against **the file** rather than against a second run of the same writer.
//!
//! # Reading a package, and modelling without one
//!
//! The suite reaches [`mjx_opc`] only to *get the bytes of a part* — `styles.xml` and, for the
//! colour-resolution cases, `theme1.xml`. Every assertion after that is made against
//! [`StylesheetPart`], which has never heard of a `PartName`: resolving a theme colour takes an
//! already-resolved [`SchemeColors`], because fetching the theme part is `mjx-xlsx`'s job and not
//! this crate's.

use mjx_dml::{ColorMap, ResolvedColor, SchemeColors, Theme};
use mjx_ooxml_core::{FromXml, RawDocument, RawNode, ToXml};
use mjx_ooxml_types::child_order::STYLESHEET;
use mjx_ooxml_types::drawingml::ColorSchemeSlot;
use mjx_ooxml_types::spreadsheetml::{
    BorderStyle, FontScheme, GradientType, HorizontalAlignment, PatternType, UnderlineType,
    VerticalAlignment,
};
use mjx_opc::{Package, PartName};
use mjx_sml::styles::palette::{resolve_color, IndexedColor, IndexedColorPalette};
use mjx_sml::{
    Color, ColorElement, DifferentialFormat, Font, FontProperties, FontPropertyOwner,
    StylesheetContent, StylesheetPart,
};

/// One committed styles part: a label, and its bytes.
struct StylesSource {
    label: String,
    bytes: Vec<u8>,
}

/// Every `styles.xml` of every committed `.xlsx` fixture, derived from the corpus directory rather
/// than from a list in this file.
fn styles_sources() -> Vec<StylesSource> {
    let mut found = Vec::new();
    for name in mjx_fixtures::all_fixture_files() {
        if !name.ends_with(".xlsx") {
            continue;
        }
        let bytes = mjx_fixtures::fixture(&name);
        let package = Package::open(&bytes).expect("a committed fixture opens");
        let parts: Vec<PartName> = package
            .part_names()
            .filter(|part| part.as_str().ends_with("/styles.xml"))
            .collect();
        for part in parts {
            found.push(StylesSource {
                label: format!("{name}::{}", part.as_str()),
                bytes: package
                    .part_bytes(&part)
                    .expect("the styles part is there")
                    .to_vec(),
            });
        }
    }
    assert!(
        found.len() >= 3,
        "only {} styles part(s) in the committed corpus — a sweep that finds nothing passes every \
         assertion below",
        found.len()
    );
    found
}

/// The bytes of one part of one committed fixture.
fn part_bytes(fixture: &str, part: &str) -> Vec<u8> {
    let bytes = mjx_fixtures::fixture(fixture);
    let package = Package::open(&bytes).expect("the fixture opens");
    let name = PartName::new(part).expect("a part name");
    package
        .part_bytes(&name)
        .expect("the part is there")
        .to_vec()
}

/// The discriminating fixture's own styles part.
fn resources() -> Vec<u8> {
    part_bytes("style_resources.xlsx", "/xl/styles.xml")
}

/// Parses `markup` and reads its `x:styleSheet`.
fn read(markup: &[u8]) -> (RawDocument, StylesheetPart) {
    let document = mjx_xml::fidelity::parse(markup).expect("the styles part parses");
    let part = StylesheetPart::read_part(&document)
        .expect("the part reads")
        .expect("the root is an x:styleSheet");
    (document, part)
}

/// Reads `markup`, writes the model straight back, and serializes — the whole model round trip.
fn round_trip(markup: &[u8]) -> Vec<u8> {
    let (mut document, part) = read(markup);
    part.write_back(&mut document.root, &mut document.interner);
    mjx_xml::fidelity::serialize_to_vec(&document)
}

/// The resolved colour scheme of a fixture's theme part.
fn scheme_of(fixture: &str) -> (RawDocument, SchemeColors) {
    let bytes = part_bytes(fixture, "/xl/theme/theme1.xml");
    let document = mjx_xml::fidelity::parse(&bytes).expect("the theme parses");
    let theme = Theme::from_xml(&document.root, &document.interner).expect("the theme reads");
    let scheme = theme
        .color_scheme()
        .map(|scheme| SchemeColors::from_scheme(scheme, &document.interner))
        .expect("the theme declares a colour scheme");
    (document, scheme)
}

// -------------------------------------------------------------------------------------------
// Tier 1 — the part re-emits byte for byte, through the model
// -------------------------------------------------------------------------------------------

/// Every committed styles part survives a full read-and-rebuild byte for byte, **and** the
/// discriminating one reads its tables back.
///
/// The second half is what stops this passing on a model that parsed into nothing.
#[test]
fn every_committed_styles_part_re_emits_byte_for_byte_and_reads_back() {
    for source in styles_sources() {
        assert_eq!(
            round_trip(&source.bytes),
            source.bytes,
            "{}: the part must come back exactly as it went in",
            source.label
        );
    }

    let (_document, part) = read(&resources());
    assert_eq!(part.fonts().expect("a font table").len(), 5);
    assert_eq!(part.fills().expect("a fill table").len(), 4);
    assert_eq!(part.borders().expect("a border table").len(), 3);
    assert_eq!(
        part.differential_formats()
            .expect("a differential-format table")
            .len(),
        4
    );
    assert!(part.colors().is_some(), "the fixture writes an x:colors");
}

/// All eleven slots are held, in the file's order, and the five modelled ones are the five this
/// child owns.
#[test]
fn all_eleven_slots_are_held_in_the_order_the_file_wrote_them() {
    let (document, part) = read(&resources());
    let locals: Vec<&str> = part.child_element_locals(&document.interner).collect();
    assert_eq!(
        locals,
        vec![
            "numFmts",
            "fonts",
            "fills",
            "borders",
            "cellStyleXfs",
            "cellXfs",
            "cellStyles",
            "dxfs",
            "tableStyles",
            "colors",
            "extLst",
        ],
        "the fixture exercises every one of CT_Stylesheet's eleven slots"
    );

    let modelled: Vec<&str> = part
        .content()
        .iter()
        .filter_map(|child| match child {
            StylesheetContent::Fonts(_) => Some("fonts"),
            StylesheetContent::Fills(_) => Some("fills"),
            StylesheetContent::Borders(_) => Some("borders"),
            StylesheetContent::DifferentialFormats(_) => Some("dxfs"),
            StylesheetContent::Colors(_) => Some("colors"),
            StylesheetContent::Raw(_) => None,
        })
        .collect();
    assert_eq!(
        modelled,
        vec!["fonts", "fills", "borders", "dxfs", "colors"],
        "MJXOFF-105 models the five resource tables; the other six are MJXOFF-108's and \
         MJXOFF-127's, held raw"
    );
    assert_eq!(STYLESHEET.slots.len(), locals.len());
}

/// The unmodelled `extLst` — prefix binding and all — survives an **edit that re-flows the part**.
///
/// Appending a font is what makes this a real assertion. Without an edit the whole part is written
/// back with every subtree's verbatim range intact, and preserving an extension takes no work at
/// all.
#[test]
fn the_foreign_extension_survives_an_edit_that_re_flows_the_part() {
    const EXTENSION: &[u8] = br#"<extLst><ext xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main" uri="{EB79DEF2-80B8-43e5-95BD-54CBDDF9020C}"><x14:slicerStyles defaultSlicerStyle="SlicerStyleLight1"/></ext></extLst>"#;
    let original = resources();
    assert!(
        windows_contains(&original, EXTENSION),
        "the fixture is supposed to carry the extension verbatim"
    );

    let (mut document, mut part) = read(&original);
    let font = Font::from_properties(
        &mut document.interner,
        None,
        &FontProperties {
            font_name: Some("Verdana".to_owned()),
            ..FontProperties::default()
        },
    )
    .expect("the font builds");
    let mut interner = core::mem::take(&mut document.interner);
    part.fonts_mut()
        .expect("a font table")
        .push(&mut interner, font);
    document.interner = interner;

    part.write_back(&mut document.root, &mut document.interner);
    let written = mjx_xml::fidelity::serialize_to_vec(&document);
    assert_ne!(written, original, "the append must have changed something");
    assert!(
        windows_contains(&written, EXTENSION),
        "the foreign extension must survive an unrelated edit, prefix binding and all"
    );
}

/// Whitespace, quoting and the self-closing choice all survive an edit — the four shapes a rebuild
/// from a decoded model cannot reproduce.
#[test]
fn the_producers_own_spelling_survives_an_edit() {
    let original = resources();
    let (mut document, mut part) = read(&original);
    let mut interner = core::mem::take(&mut document.interner);
    let border = mjx_sml::Border::new(&mut interner, None);
    part.borders_mut()
        .expect("a border table")
        .push(&mut interner, border);
    document.interner = interner;
    part.write_back(&mut document.root, &mut document.interner);
    let written = mjx_xml::fidelity::serialize_to_vec(&document);

    for (shape, why) in [
        (
            &br#"<fonts count='5'>"#[..],
            "a single-quoted attribute on a modelled container",
        ),
        (
            &br#"<cellStyle name="Normal"  xfId="0" builtinId="0"/>"#[..],
            "a doubled space inside an unmodelled slot's start tag",
        ),
        (
            &br#"<top style="hair"></top>"#[..],
            "an element the file wrote open-and-close rather than self-closed",
        ),
        (
            &b"<!-- the resource tables end here; the xf indirection below is MJXOFF-108's -->"[..],
            "a comment between two slots",
        ),
        (
            &br#"formatCode="0.000&quot;m&quot;""#[..],
            "an entity reference inside an attribute value",
        ),
    ] {
        assert!(
            windows_contains(&original, shape),
            "the fixture is supposed to carry {why}"
        );
        assert!(
            windows_contains(&written, shape),
            "{why} must survive an edit elsewhere in the part"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Tier 2 — the values read back
// -------------------------------------------------------------------------------------------

/// Every font in the table reads back through `FontProperties` — the same type a rich-text run's
/// `rPr` decodes to, not a second one.
#[test]
fn every_font_reads_back_through_the_shared_property_family() {
    let (document, part) = read(&resources());
    let interner = &document.interner;
    let fonts = part.fonts().expect("a font table");

    let first = fonts.get(0).expect("font 0").properties(interner);
    assert_eq!(first.font_name.as_deref(), Some("Calibri"));
    assert_eq!(first.size_in_points, Some(11.0));
    assert_eq!(first.family, Some(2));
    assert_eq!(first.scheme, Some(FontScheme::Minor));
    assert_eq!(first.color.as_ref().and_then(|color| color.theme), Some(1));
    assert_eq!(first.bold, None, "an absent `b` is not a `b` set to false");

    // Font 1 sets all fifteen slots, including the entity in its name and the seven boolean
    // properties in both polarities.
    let full = fonts.get(1).expect("font 1").properties(interner);
    assert_eq!(full.font_name.as_deref(), Some("Cambria & Co"));
    assert_eq!(full.character_set, Some(1));
    assert_eq!(full.family, Some(1));
    assert_eq!(full.bold, Some(true));
    assert_eq!(full.italic, Some(false));
    assert_eq!(full.strikethrough, Some(true));
    assert_eq!(full.outline, Some(false));
    assert_eq!(full.shadow, Some(true));
    assert_eq!(full.condensed, Some(false));
    assert_eq!(full.extended, Some(true));
    assert_eq!(full.size_in_points, Some(13.5));
    assert_eq!(full.underline, Some(UnderlineType::DoubleAccounting));
    assert_eq!(
        full.vertical_position,
        Some(mjx_ooxml_types::shared::VerticalTextPosition::Superscript)
    );
    assert_eq!(full.scheme, Some(FontScheme::Major));
    assert_eq!(
        full.color.as_ref().and_then(|color| color.rgb.as_deref()),
        Some("FF18A303")
    );
    assert!(
        full.extra.is_empty(),
        "every one of CT_Font's fifteen children is modelled by FontProperties, so nothing should \
         land in the unknown bucket"
    );

    let indexed = fonts.get(4).expect("font 4").properties(interner);
    assert_eq!(
        indexed.color.as_ref().and_then(|color| color.indexed),
        Some(10)
    );
    assert_eq!(indexed.font_name.as_deref(), Some("Courier New"));
}

/// A font-table entry and a rich-text run holding the same properties decode to the **same value** —
/// the reuse claim, asserted rather than asserted about.
#[test]
fn a_font_table_entry_and_a_run_decode_to_one_type() {
    let run = FontProperties::from_markup(
        br#"<rPr><sz val="10"/><rFont val="Arial"/><family val="2"/></rPr>"#,
        FontPropertyOwner::RichTextRun,
    )
    .expect("the run parses");

    let (document, part) = read(&resources());
    let entry = part
        .fonts()
        .expect("a font table")
        .get(2)
        .expect("font 2")
        .properties(&document.interner);
    assert_eq!(entry, run);
}

/// Both kinds of fill read back, including the gradient's six attributes and its two stops.
#[test]
fn every_fill_reads_back_including_the_gradient() {
    let (document, part) = read(&resources());
    let interner = &document.interner;
    let fills = part.fills().expect("a fill table");

    let none = fills.get(0).expect("fill 0").pattern().expect("a pattern");
    assert_eq!(
        none.pattern_type(interner).expect("a type"),
        Some(PatternType::None)
    );
    assert_eq!(none.foreground_color_element(), None);

    let two_colour = fills.get(2).expect("fill 2").pattern().expect("a pattern");
    assert_eq!(
        two_colour.pattern_type(interner).expect("a type"),
        Some(PatternType::DarkTrellis)
    );
    assert_eq!(
        two_colour
            .foreground_colour(interner)
            .and_then(|color| color.rgb),
        Some("FFFFFF00".to_owned()),
        "a solid or patterned fill's visible colour is the FOREGROUND one"
    );
    assert_eq!(
        two_colour
            .background_colour(interner)
            .and_then(|color| color.indexed),
        Some(64)
    );

    let gradient = fills
        .get(3)
        .expect("fill 3")
        .gradient()
        .expect("a gradient");
    assert!(
        fills.get(3).expect("fill 3").pattern().is_none(),
        "CT_Fill is a choice: a gradient fill is not also a pattern fill"
    );
    assert_eq!(
        gradient.gradient_type(interner).expect("a type"),
        GradientType::Path
    );
    assert_eq!(gradient.degrees(interner).expect("a degree"), 45.0);
    assert_eq!(gradient.left_inset(interner).expect("left"), 0.2);
    assert_eq!(gradient.right_inset(interner).expect("right"), 0.8);
    assert_eq!(gradient.top_inset(interner).expect("top"), 0.1);
    assert_eq!(gradient.bottom_inset(interner).expect("bottom"), 0.9);

    let stops: Vec<(f64, Color)> = gradient
        .stops()
        .map(|stop| {
            (
                stop.position(interner)
                    .expect("a position")
                    .expect("written"),
                stop.colour(interner).expect("a colour"),
            )
        })
        .collect();
    assert_eq!(stops.len(), 2);
    assert_eq!(stops[0].0, 0.0);
    assert_eq!(stops[0].1.theme, Some(4));
    assert_eq!(stops[0].1.tint, Some(-0.25));
    assert_eq!(stops[1].0, 1.0);
    assert_eq!(stops[1].1.rgb.as_deref(), Some("FF0369A3"));
}

/// All **nine** edges of a border read back — `start` and `end` included, which the ticket's "six
/// edges plus diagonal" would have left out.
#[test]
fn all_nine_border_edges_read_back() {
    let (document, part) = read(&resources());
    let interner = &document.interner;
    let borders = part.borders().expect("a border table");

    let plain = borders.get(0).expect("border 0");
    assert_eq!(
        plain
            .top_edge()
            .expect("a top edge")
            .style(interner)
            .expect("a style"),
        BorderStyle::None,
        "`<top/>` states the schema default `none`; it is a value, not a silence"
    );
    assert_eq!(plain.leading_edge(), None, "border 0 writes no `start`");

    let full = borders.get(1).expect("border 1");
    let styles: Vec<(&str, BorderStyle)> = vec![
        (
            "start",
            full.leading_edge()
                .expect("start")
                .style(interner)
                .expect("style"),
        ),
        (
            "end",
            full.trailing_edge()
                .expect("end")
                .style(interner)
                .expect("style"),
        ),
        (
            "left",
            full.left_edge()
                .expect("left")
                .style(interner)
                .expect("style"),
        ),
        (
            "right",
            full.right_edge()
                .expect("right")
                .style(interner)
                .expect("style"),
        ),
        (
            "top",
            full.top_edge()
                .expect("top")
                .style(interner)
                .expect("style"),
        ),
        (
            "bottom",
            full.bottom_edge()
                .expect("bottom")
                .style(interner)
                .expect("style"),
        ),
        (
            "diagonal",
            full.diagonal_edge()
                .expect("diagonal")
                .style(interner)
                .expect("style"),
        ),
        (
            "vertical",
            full.vertical_inner_edge()
                .expect("vertical")
                .style(interner)
                .expect("style"),
        ),
        (
            "horizontal",
            full.horizontal_inner_edge()
                .expect("horizontal")
                .style(interner)
                .expect("style"),
        ),
    ];
    assert_eq!(
        styles,
        vec![
            ("start", BorderStyle::Thin),
            ("end", BorderStyle::Thick),
            ("left", BorderStyle::Medium),
            ("right", BorderStyle::MediumDashDotDot),
            ("top", BorderStyle::Hair),
            ("bottom", BorderStyle::Double),
            ("diagonal", BorderStyle::SlantDashDot),
            ("vertical", BorderStyle::Dotted),
            ("horizontal", BorderStyle::DashDotDot),
        ],
        "CT_Border declares nine edges and every one of them must be reachable"
    );

    assert_eq!(full.diagonal_up(interner).expect("diagonalUp"), Some(true));
    assert_eq!(
        full.diagonal_down(interner).expect("diagonalDown"),
        Some(false)
    );
    assert!(
        !full.outline_only(interner).expect("outline"),
        "border 1 writes `outline=\"false\"`"
    );
    assert!(
        plain.outline_only(interner).expect("outline"),
        "`@outline` defaults to TRUE, which is the opposite of what absent-means-off would give"
    );

    // Every one of the five colour slots decodes through the one CT_Color type.
    assert_eq!(
        full.leading_edge()
            .and_then(|edge| edge.colour(interner))
            .and_then(|color| color.rgb),
        Some("FF000000".to_owned())
    );
    assert_eq!(
        full.left_edge()
            .and_then(|edge| edge.colour(interner))
            .and_then(|color| color.indexed),
        Some(8)
    );
    assert_eq!(
        full.right_edge()
            .and_then(|edge| edge.colour(interner))
            .and_then(|color| color.tint),
        Some(0.5)
    );
    assert_eq!(
        full.bottom_edge()
            .and_then(|edge| edge.colour(interner))
            .and_then(|color| color.automatic),
        Some(true)
    );
    assert_eq!(
        full.trailing_edge().expect("end").colour(interner),
        None,
        "an edge with no `color` child is *automatic*, which is an absence and not a colour"
    );
}

/// An absent `dxf` member means **inherit**, and that is distinguishable from every member set to
/// its default.
#[test]
fn an_absent_dxf_member_is_inherited_and_not_defaulted() {
    let (document, part) = read(&resources());
    let interner = &document.interner;
    let formats = part.differential_formats().expect("a dxf table");

    let fill_only = formats.get(0).expect("dxf 0");
    assert!(fill_only.fill().is_some(), "dxf 0 sets a fill");
    for (member, present) in [
        ("font", fill_only.font().is_some()),
        ("numFmt", fill_only.number_format().is_some()),
        ("alignment", fill_only.alignment().is_some()),
        ("border", fill_only.border().is_some()),
        ("protection", fill_only.protection().is_some()),
    ] {
        assert!(
            !present,
            "dxf 0 states only a fill; `{member}` must read as inherited, not as a default value"
        );
    }
    assert!(!fill_only.inherits_everything());

    let font_only = formats.get(1).expect("dxf 1");
    assert!(font_only.fill().is_none() && font_only.font().is_some());

    let everything = formats.get(2).expect("dxf 2");
    assert!(everything.font().is_some());
    assert_eq!(
        everything
            .number_format()
            .expect("numFmt")
            .format_code(interner)
            .expect("a format code")
            .as_deref(),
        Some("0.00%")
    );
    assert_eq!(
        everything
            .alignment()
            .expect("alignment")
            .horizontal_alignment(interner)
            .expect("horizontal")
            .expect("written"),
        HorizontalAlignment::Center
    );
    assert_eq!(
        everything
            .alignment()
            .expect("alignment")
            .vertical_alignment(interner)
            .expect("vertical"),
        VerticalAlignment::Top
    );
    assert_eq!(
        everything
            .protection()
            .expect("protection")
            .formula_hidden(interner)
            .expect("hidden"),
        Some(true)
    );
    assert_eq!(
        everything
            .border()
            .expect("border")
            .left_edge()
            .expect("left")
            .style(interner)
            .expect("a style"),
        BorderStyle::Thin
    );

    let inherited = formats.get(3).expect("dxf 3");
    assert!(
        inherited.inherits_everything(),
        "`<dxf/>` is a meaningful value: inherit everything"
    );
    assert!(inherited.fill().is_none() && inherited.font().is_none());

    // And the two are not the same value, which is the whole point of the type.
    assert_ne!(fill_only, inherited);
}

// -------------------------------------------------------------------------------------------
// Index identity
// -------------------------------------------------------------------------------------------

/// A font's identity is its **position**, and appending never moves an existing one — even when the
/// appended font is byte-identical to one already in the table.
///
/// The fixture writes fonts 2 and 3 identically on purpose, so deduplicating on write breaks this
/// case rather than passing it.
#[test]
fn font_indices_are_positions_and_appending_never_moves_them() {
    let original = resources();
    let (mut document, mut part) = read(&original);

    let before: Vec<FontProperties> = part
        .fonts()
        .expect("a font table")
        .fonts()
        .map(|font| font.properties(&document.interner))
        .collect();
    assert_eq!(before.len(), 5);
    assert_eq!(
        before[2], before[3],
        "the fixture is supposed to carry two identical fonts; without them nothing here can \
         detect deduplication"
    );

    // Append a font identical to the one already at index 0.
    let clone_of_first =
        Font::from_properties(&mut document.interner, None, &before[0]).expect("the font builds");
    let mut interner = core::mem::take(&mut document.interner);
    part.fonts_mut()
        .expect("a font table")
        .push(&mut interner, clone_of_first);
    document.interner = interner;

    let after: Vec<FontProperties> = part
        .fonts()
        .expect("a font table")
        .fonts()
        .map(|font| font.properties(&document.interner))
        .collect();
    assert_eq!(
        after.len(),
        6,
        "appending a font identical to an existing one must still add an entry: an `xf` pointing \
         at index 5 has to find it"
    );
    assert_eq!(
        &after[..5],
        &before[..],
        "every existing index must still name exactly the font it named before"
    );
    assert_eq!(after[5], before[0]);
    assert_eq!(
        after[2], after[3],
        "the identical pair must still be two entries, at two indices"
    );

    // And the same holds after a write and a read.
    part.write_back(&mut document.root, &mut document.interner);
    let written = mjx_xml::fidelity::serialize_to_vec(&document);
    let (reread_document, reread) = read(&written);
    let round_tripped: Vec<FontProperties> = reread
        .fonts()
        .expect("a font table")
        .fonts()
        .map(|font| font.properties(&reread_document.interner))
        .collect();
    assert_eq!(
        round_tripped, after,
        "a write must not deduplicate, reorder or drop an entry — every index in the workbook \
         would change meaning"
    );
}

/// `@count` moves with an append when the file declared one, and a table that declared none stays
/// that way.
#[test]
fn the_declared_count_moves_with_an_append_and_is_not_invented() {
    let (mut document, mut part) = read(&resources());
    {
        let interner = &document.interner;
        assert_eq!(
            part.fonts()
                .expect("a font table")
                .declared_count(interner)
                .expect("a count"),
            Some(5)
        );
    }
    let font = Font::from_properties(
        &mut document.interner,
        None,
        &FontProperties {
            font_name: Some("Verdana".to_owned()),
            ..FontProperties::default()
        },
    )
    .expect("the font builds");
    let mut interner = core::mem::take(&mut document.interner);
    part.fonts_mut()
        .expect("a font table")
        .push(&mut interner, font);
    document.interner = interner;
    assert_eq!(
        part.fonts()
            .expect("a font table")
            .declared_count(&document.interner)
            .expect("a count"),
        Some(6),
        "`@count` has to move with the table, or the file says something untrue about itself"
    );

    // A table that declared no count keeps none: an absent optional attribute is the producer's
    // choice, and inventing one would author markup nobody asked for.
    let markup = concat!(
        r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
        r#"<fonts><font><name val="Arial"/></font></fonts>"#,
        "</styleSheet>"
    );
    let (mut document, mut part) = read(markup.as_bytes());
    let font = Font::from_properties(
        &mut document.interner,
        None,
        &FontProperties {
            font_name: Some("Verdana".to_owned()),
            ..FontProperties::default()
        },
    )
    .expect("the font builds");
    let mut interner = core::mem::take(&mut document.interner);
    part.fonts_mut()
        .expect("a font table")
        .push(&mut interner, font);
    document.interner = interner;
    assert_eq!(
        part.fonts()
            .expect("a font table")
            .declared_count(&document.interner)
            .expect("a count"),
        None
    );
    assert_eq!(part.fonts().expect("a font table").len(), 2);
}

// -------------------------------------------------------------------------------------------
// Tier 3 — resolution
// -------------------------------------------------------------------------------------------

/// A theme colour resolves to exactly what `mjx-dml` resolves for the same scheme slot — asserted
/// against `resolve_color`, never against a hard-coded hex.
///
/// SpreadsheetML addresses a theme colour by *position*; DrawingML addresses it by *token*. The two
/// must land on one colour, and ECMA-376 Part 1 §20.1.6.2's index table is what maps between them.
#[test]
fn a_theme_colour_resolves_to_what_drawingml_resolves_for_the_same_slot() {
    let (theme_document, scheme) = scheme_of("style_resources.xlsx");
    let theme =
        Theme::from_xml(&theme_document.root, &theme_document.interner).expect("the theme reads");
    let color_scheme = theme.color_scheme().expect("a colour scheme");
    let palette = IndexedColorPalette::default_palette();

    for (position, slot) in [
        (0, ColorSchemeSlot::Dark1),
        (1, ColorSchemeSlot::Light1),
        (2, ColorSchemeSlot::Dark2),
        (3, ColorSchemeSlot::Light2),
        (4, ColorSchemeSlot::Accent1),
        (9, ColorSchemeSlot::Accent6),
        (10, ColorSchemeSlot::Hyperlink),
        (11, ColorSchemeSlot::FollowedHyperlink),
    ] {
        let through_drawingml = mjx_dml::resolve_color(
            color_scheme.color(slot).expect("the slot is defined"),
            &SchemeColors::default(),
            &ColorMap::identity(),
            None,
            &theme_document.interner,
        )
        .expect("a DrawingML colour resolves");
        let through_spreadsheetml =
            resolve_color(&Color::from_theme(position, None), &scheme, &palette)
                .expect("a SpreadsheetML theme colour resolves");
        assert_eq!(
            through_spreadsheetml.to_hex(),
            through_drawingml.to_hex(),
            "theme=\"{position}\" is {slot:?} per Part 1 §20.1.6.2, and the two must agree"
        );
    }

    // A tint moves the colour, and moves it the way §18.8.19 says.
    let base = resolve_color(&Color::from_theme(4, None), &scheme, &palette).expect("accent1");
    let lightened =
        resolve_color(&Color::from_theme(4, Some(0.6)), &scheme, &palette).expect("accent1 tinted");
    let darkened = resolve_color(&Color::from_theme(4, Some(-0.6)), &scheme, &palette)
        .expect("accent1 shaded");
    assert_ne!(base, lightened);
    assert!(
        brightness(lightened) > brightness(base) && brightness(darkened) < brightness(base),
        "a positive tint lightens and a negative one darkens: {base:?} {lightened:?} {darkened:?}"
    );
    assert_eq!(
        resolve_color(&Color::from_theme(4, Some(0.0)), &scheme, &palette),
        Some(base),
        "`tint=\"0\"` is the schema default and must change nothing"
    );
}

/// An indexed colour resolves through the default palette, and through an overriding
/// `indexedColors` block when the workbook writes one.
#[test]
fn an_indexed_colour_resolves_through_the_palette_the_workbook_declares() {
    let (document, part) = read(&resources());
    let interner = &document.interner;
    let scheme = SchemeColors::default();

    let default_palette = IndexedColorPalette::default_palette();
    let declared = part
        .colors()
        .expect("an x:colors")
        .indexed_colors()
        .expect("the fixture writes an indexedColors");
    let override_palette = IndexedColorPalette::from_indexed_colors(declared, interner);

    assert_eq!(override_palette.len(), 64);
    assert!(!override_palette.is_default());

    // The fixture's palette differs from the default at exactly one row.
    let differing: Vec<u32> = (0..64)
        .filter(|index| default_palette.lookup(*index) != override_palette.lookup(*index))
        .collect();
    assert_eq!(
        differing,
        vec![10],
        "the fixture replaces exactly one row, so an implementation that ignored the override \
         would answer differently at that row and nowhere else"
    );

    let indexed_ten = Color {
        indexed: Some(10),
        ..Color::default()
    };
    assert_eq!(
        resolve_color(&indexed_ten, &scheme, &default_palette)
            .expect("the default palette defines index 10")
            .to_hex(),
        "FF0000",
        "Part 1 §18.8.27 row 10 is 00FF0000"
    );
    assert_eq!(
        resolve_color(&indexed_ten, &scheme, &override_palette)
            .expect("the override defines index 10")
            .to_hex(),
        "1A2B3C",
        "the workbook's own palette wins where it differs"
    );

    // A row the override leaves alone still agrees with the default.
    let indexed_twenty_two = Color {
        indexed: Some(22),
        ..Color::default()
    };
    assert_eq!(
        resolve_color(&indexed_twenty_two, &scheme, &override_palette),
        resolve_color(&indexed_twenty_two, &scheme, &default_palette)
    );

    // And the two system indices are not colours in either palette.
    for index in [64, 65] {
        let system = Color {
            indexed: Some(index),
            ..Color::default()
        };
        assert_eq!(resolve_color(&system, &scheme, &default_palette), None);
        assert_eq!(
            default_palette.lookup(index),
            Some(if index == 64 {
                IndexedColor::SystemForeground
            } else {
                IndexedColor::SystemBackground
            })
        );
    }

    // The fixture's own `bgColor indexed="64"` is one of them, which is why it must be reported
    // rather than resolved to a hex.
    let background = part
        .fills()
        .expect("a fill table")
        .get(2)
        .expect("fill 2")
        .pattern()
        .expect("a pattern")
        .background_colour(interner)
        .expect("a bgColor");
    assert_eq!(background.indexed, Some(64));
    assert_eq!(resolve_color(&background, &scheme, &override_palette), None);
}

/// One `CT_Color` element type stands in all five slots, and every one of them decodes through
/// [`Color`].
#[test]
fn one_colour_element_type_serves_every_slot() {
    let (document, part) = read(&resources());
    let interner = &document.interner;

    let font_color: Option<Color> = part
        .fonts()
        .expect("fonts")
        .get(0)
        .expect("font 0")
        .properties(interner)
        .color;
    let foreground = part
        .fills()
        .expect("fills")
        .get(2)
        .expect("fill 2")
        .pattern()
        .expect("a pattern")
        .foreground_color_element()
        .expect("an fgColor");
    let background = part
        .fills()
        .expect("fills")
        .get(2)
        .expect("fill 2")
        .pattern()
        .expect("a pattern")
        .background_color_element()
        .expect("a bgColor");
    let edge = part
        .borders()
        .expect("borders")
        .get(1)
        .expect("border 1")
        .leading_edge()
        .expect("start")
        .color_element()
        .expect("a colour");
    let stop = part
        .fills()
        .expect("fills")
        .get(3)
        .expect("fill 3")
        .gradient()
        .expect("a gradient")
        .stops()
        .next()
        .expect("a stop")
        .color_element()
        .expect("a colour");
    let mru = part
        .colors()
        .expect("colors")
        .most_recently_used()
        .expect("mruColors")
        .color_elements()
        .next()
        .expect("an entry");

    // Four `ColorElement`s under three different local names, all decoding through one type.
    let locals: Vec<&str> = [foreground, background, edge, stop, mru]
        .iter()
        .map(|element| interner.resolve(element.element_name().local))
        .collect();
    assert_eq!(
        locals,
        vec!["fgColor", "bgColor", "color", "color", "color"],
        "each element keeps the name its slot gives it; the type does not name itself"
    );
    assert!(font_color.is_some());
    assert_eq!(edge.color(interner).rgb.as_deref(), Some("FF000000"));
    assert_eq!(mru.color(interner).rgb.as_deref(), Some("FF18A303"));

    // And an authored one writes the same attributes the reader decodes.
    let mut interner = mjx_ooxml_core::Interner::default();
    let authored = ColorElement::named(
        &mut interner,
        None,
        "fgColor",
        &Color::from_theme(4, Some(-0.25)),
    );
    let decoded = authored.color(&interner);
    assert_eq!(decoded.theme, Some(4));
    assert_eq!(decoded.tint, Some(-0.25));
}

/// A `dxf` and a `styleSheet` built from nothing place their children at the schema's ranks.
#[test]
fn an_authored_differential_format_places_its_members_by_rank() {
    let mut interner = mjx_ooxml_core::Interner::default();
    let mut format = DifferentialFormat::new(&mut interner, None);
    // Set them in the reverse of schema order; placement has to sort it out.
    format.set_protection(Some(mjx_sml::CellProtection::new(&mut interner, None)));
    format.set_font(Some(
        Font::from_properties(
            &mut interner,
            None,
            &FontProperties {
                bold: Some(true),
                ..FontProperties::default()
            },
        )
        .expect("the font builds"),
    ));
    format.set_border(Some(mjx_sml::Border::new(&mut interner, None)));
    format.set_alignment(Some(mjx_sml::CellAlignment::new(&mut interner, None)));

    let element = format.to_xml(&mut interner);
    let locals: Vec<&str> = element
        .children
        .iter()
        .filter_map(|node| match node {
            RawNode::Element(child) => Some(interner.resolve(child.name.local)),
            _ => None,
        })
        .collect();
    assert_eq!(
        locals,
        vec!["font", "alignment", "border", "protection"],
        "CT_Dxf's sequence is font, numFmt, fill, alignment, border, protection, extLst"
    );
}

// -------------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------------

/// Whether `haystack` contains `needle`.
fn windows_contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The sum of a resolved colour's channels — enough to say "lighter" and "darker" without pinning a
/// hex this suite would then be asserting against itself.
fn brightness(color: ResolvedColor) -> u32 {
    u32::from(color.red) + u32::from(color.green) + u32::from(color.blue)
}
