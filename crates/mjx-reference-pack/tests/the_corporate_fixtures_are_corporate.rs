//! **The corporate fixtures are what RC03 says they are** (MJXOFF-300).
//!
//! Every fixture committed before this ticket was written to break an algorithm: a hairline border,
//! a style chain that loops, a declared size that lies. None of them looks like a file somebody
//! would send a colleague, and a renderer can pass all of them while producing something no reader
//! would accept. RC03 commits three that do look like that, and this suite is what stops them
//! quietly becoming ordinary fixtures again.
//!
//! It asks three things of each one:
//!
//! 1. **It is registered.** `mjx-fixtures` derives the corpus from the directory, so a fixture joins
//!    every byte-identity suite and the schema gate by *existing* — and this asserts it by name, so
//!    a fixture deleted or renamed fails here rather than silently leaving those corpora.
//! 2. **It opens, and the container round-trips byte-identically** — the promise the whole project
//!    exists for, asked of the three files most likely to be edited by hand later.
//! 3. **It carries the elements the ticket lists**, one assertion per element, so a fixture missing
//!    one fails *by name* instead of by a render that looks subtly wrong three tickets later.
//!
//! # ⚠ Why some elements are asserted as markup rather than through a typed reader
//!
//! The ticket asks for the typed model, and where a reader exists that is what is used — a custom
//! geometry through [`mjx_pptx::Geometry`], a table style through `Presentation::table_style_id`, a
//! tracked change through `Document::revisions`. **Three of the elements it names have no typed
//! reader in this workspace at all**, and each is recorded as such beside the probe that stands in
//! for it:
//!
//! * a picture's crop (`a:srcRect`) — `mjx_dml::FillSpec::Picture` carries `rel_id` and `mode` and
//!   no source rectangle;
//! * a slide or master **background** (`p:bg`) — `mjx-pptx` has no background accessor;
//! * a paragraph's **ordered content** (`a:br` between runs) — which `features.json` already records
//!   as *"not public yet"* on the `pptx-line-breaks` row.
//!
//! A markup probe is a weaker assertion than a typed one and is written here as a **placeholder for
//! a reader**, not as a preference. Each one names the reader that would replace it.

use std::collections::BTreeSet;

use mjx_dml::{GuideContext, ShapeGeometry, Size};
use mjx_docx::Document;
use mjx_ooxml_types::presentationml::PlaceholderType;
use mjx_opc::Package;
use mjx_pptx::{Geometry, Presentation, Surface};
use mjx_reference_pack::outlines::shape_outline;
use mjx_xlsx::Workbook;

/// The three fixtures RC03 commits.
const CORPORATE: [(&str, &str); 3] = [
    ("pptx", "corporate.pptx"),
    ("xlsx", "corporate.xlsx"),
    ("docx", "corporate.docx"),
];

/// One element the ticket names, and the markup that proves the fixture carries it.
struct MarkupProbe {
    /// What the fixture must carry, in the ticket's own words.
    element: &'static str,
    /// The exact markup that proves it. Never a guess: each is the element's own spelling.
    needle: &'static str,
    /// The typed reader that would replace this probe, or why none is wanted.
    instead_of: &'static str,
}

/// The PowerPoint elements, as markup.
const DECK_PROBES: &[MarkupProbe] = &[
    MarkupProbe {
        element: "a gradient background on the master",
        needle: "<a:gradFill",
        instead_of:
            "no background reader exists on `mjx_pptx::Presentation` (`p:bg` is unmodelled \
                     at the surface)",
    },
    MarkupProbe {
        element: "a soft line break inside the title",
        needle: "<a:br",
        instead_of:
            "the ordered paragraph-content reader `features.json`'s `pptx-line-breaks` row \
                     records as not public yet",
    },
    MarkupProbe {
        element: "a cropped picture",
        needle: "<a:srcRect",
        instead_of: "`mjx_dml::FillSpec::Picture` carries `rel_id` and `mode` and no source \
                     rectangle",
    },
    MarkupProbe {
        element: "an ellipse-masked picture",
        needle: "prst=\"ellipse\"",
        instead_of: "`Presentation::shape_geometry` reads a shape's geometry; a `p:pic`'s mask is \
                     the same element and is asserted here beside the crop it travels with",
    },
    MarkupProbe {
        element: "an arrow connector",
        needle: "<p:cxnSp",
        instead_of: "`Presentation::shape_kind` names a shape's kind but no accessor answers a \
                     connector's line ends",
    },
    MarkupProbe {
        element: "an arrowhead on that connector",
        needle: "tailEnd",
        instead_of: "as above",
    },
    MarkupProbe {
        element: "a semi-transparent overlay rectangle",
        needle: "<a:alpha",
        instead_of:
            "`Presentation::shape_fill` answers the fill; the alpha transform on its colour \
                     has no accessor of its own",
    },
    MarkupProbe {
        element: "a bulleted list with Wingdings bullets",
        needle: "Wingdings",
        instead_of:
            "`Presentation::shape_list_style_level` answers a level's properties; the bullet \
                     font has no accessor of its own",
    },
    MarkupProbe {
        element: "the cached drawing of the SmartArt frame",
        needle: "<dsp:drawing",
        instead_of: "`Presentation::diagram_parts` resolves the four parts; nothing reads the \
                     cached drawing's content",
    },
];

/// The Excel elements, as markup.
const SHEET_PROBES: &[MarkupProbe] = &[
    MarkupProbe {
        element: "a colour scale",
        needle: "<colorScale",
        instead_of: "`Workbook::conditional_rules_for` answers a chain per cell; this asserts the \
                     rule exists at all without naming a cell the fixture has not chosen yet",
    },
    MarkupProbe {
        element: "a data bar",
        needle: "<dataBar",
        instead_of: "as above",
    },
    MarkupProbe {
        element: "an icon set",
        needle: "<iconSet",
        instead_of: "as above",
    },
    MarkupProbe {
        element: "an accounting number format",
        needle: "_(",
        instead_of:
            "`mjx_sml`'s number-format table answers by index; the accounting format's own \
                     code is what makes it accounting",
    },
    MarkupProbe {
        element: "a diagonal border",
        needle: "<diagonal style=",
        instead_of:
            "`Workbook::effective_cell_format` answers a cell's border; the diagonal is the \
                     one edge `mjx_scene::Decoration` cannot carry, so it is asserted in the file",
    },
    MarkupProbe {
        element: "a dashed border",
        needle: "style=\"dashed\"",
        instead_of: "as above",
    },
    MarkupProbe {
        element: "a frozen pane",
        needle: "state=\"frozen\"",
        instead_of: "`mjx_layout_xlsx::PaneSplit` reads it at the layout tier; the typed workbook \
                     surface has no per-sheet view accessor",
    },
    MarkupProbe {
        element: "a rich-text cell",
        needle: "<rPr>",
        instead_of:
            "`Workbook::shared_strings` answers the strings; a run's own properties inside \
                     one have no accessor",
    },
];

/// The Word elements, as markup.
const DOCUMENT_PROBES: &[MarkupProbe] = &[
    MarkupProbe {
        element: "a style carrying a theme colour",
        needle: "w:themeColor",
        instead_of: "`Document::style_sheet` answers the styles; the theme reference is resolved \
                     before it reaches a consumer, so the file is where it is still visible",
    },
    MarkupProbe {
        element: "a style carrying a themeShade",
        needle: "w:themeShade",
        instead_of: "as above",
    },
    MarkupProbe {
        element: "a banded header row on the styled table",
        needle: "w:tblStyle",
        instead_of:
            "`Document::tables` answers the tables; the style reference has no accessor of \
                     its own",
    },
    MarkupProbe {
        element: "an inline picture",
        needle: "<wp:inline",
        instead_of:
            "`DocumentFormatting::drawings` answers the drawings; inline versus floating is \
                     the anchor element itself",
    },
    MarkupProbe {
        element: "a floating picture",
        needle: "<wp:anchor",
        instead_of: "as above",
    },
    MarkupProbe {
        element: "a text box",
        needle: "<w:txbxContent",
        instead_of:
            "`DrawingFormatting::text_box` answers one; this asserts the fixture has one to \
                     answer about",
    },
    MarkupProbe {
        element: "a footnote reference",
        needle: "<w:footnoteReference",
        instead_of: "`Document::footnotes` answers the notes part; the reference in the body is \
                     what puts one on the page",
    },
    MarkupProbe {
        element: "a bulleted list with Symbol bullets",
        needle: "Symbol",
        instead_of:
            "`Document::numbering` answers the definitions; the bullet font has no accessor \
                     of its own",
    },
    MarkupProbe {
        element: "an equation",
        needle: "<m:oMath",
        instead_of: "`DocumentFormatting::equations` answers them; asserted both ways below",
    },
    MarkupProbe {
        element: "a page border",
        needle: "<w:pgBorders",
        instead_of: "`SectionFormatting` carries page size, margins and columns and no page border",
    },
];

/// Every XML payload in a package, concatenated, so an element can be asserted by its own spelling.
fn package_markup(bytes: &[u8]) -> String {
    let package = Package::open(bytes).expect("the fixture is a well-formed package");
    let mut all = String::new();
    for entry in package.entries() {
        if !entry.name.ends_with(".xml") && !entry.name.ends_with(".rels") {
            continue;
        }
        // `None` is an edited part, which a committed fixture has none of.
        if let Some(bytes) = entry.bytes() {
            all.push_str(&String::from_utf8_lossy(bytes));
            all.push('\n');
        }
    }
    assert!(
        !all.is_empty(),
        "the package holds no XML at all, so every probe below would fail for the wrong reason"
    );
    all
}

/// Asserts every probe finds its element, naming the one that does not.
fn assert_probes(fixture: &str, probes: &[MarkupProbe]) {
    let markup = package_markup(&mjx_fixtures::fixture(fixture));
    let missing: Vec<String> = probes
        .iter()
        .filter(|probe| !markup.contains(probe.needle))
        .map(|probe| {
            format!(
                "{} — markup `{}`, standing in for {}",
                probe.element, probe.needle, probe.instead_of
            )
        })
        .collect();
    assert!(
        missing.is_empty(),
        "{fixture} is missing {} of the elements MJXOFF-300 lists: {missing:?}. Each is asserted by \
         its own markup; see this suite's own documentation for why a probe rather than a reader.",
        missing.len()
    );
}

/// **Each fixture is registered**, which for this corpus means committed to `tests/fixtures/`.
#[test]
fn every_corporate_fixture_is_in_the_committed_corpus() {
    mjx_fixtures::assert_every_fixture_has_a_known_kind();
    for (extension, name) in CORPORATE {
        let corpus = mjx_fixtures::package_fixtures_with_extension(extension);
        assert!(
            corpus.iter().any(|fixture| fixture == name),
            "`{name}` is not in the committed `.{extension}` corpus, which `mjx-fixtures` derives \
             from `tests/fixtures/`. Every byte-identity suite and the schema gate sweep that \
             corpus, so a corporate fixture outside it is a fixture nothing holds to the round-trip \
             contract. The corpus today: {corpus:?}"
        );
    }
}

/// **Each fixture round-trips byte-identically at the container**, which is the project's promise.
#[test]
fn every_corporate_fixture_round_trips_byte_identically() {
    for (_, name) in CORPORATE {
        let original = mjx_fixtures::fixture(name);
        let package = Package::open(&original).unwrap_or_else(|e| panic!("{name}: open: {e}"));
        let saved = package
            .save()
            .unwrap_or_else(|e| panic!("{name}: save: {e}"));
        let reopened = Package::open(&saved).unwrap_or_else(|e| panic!("{name}: reopen: {e}"));

        let before: Vec<&str> = package.entries().iter().map(|e| e.name.as_str()).collect();
        let after: Vec<&str> = reopened.entries().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            before, after,
            "{name}: the entry set or its order changed across a round trip"
        );

        for (a, b) in package.entries().iter().zip(reopened.entries()) {
            assert_eq!(
                a.bytes(),
                b.bytes(),
                "{name}: the decompressed bytes of entry {} changed across a round trip",
                a.name
            );
        }

        // A package of one entry would satisfy everything above. A corporate file is not that.
        assert!(
            package.entries().len() >= 8,
            "{name}: {} entries. A file with a theme, a picture and a chart in it has more parts \
             than that, and a round-trip assertion over a near-empty container proves nothing.",
            package.entries().len()
        );
    }
}

/// The deck carries the elements MJXOFF-300 lists, through the typed model where one exists.
#[test]
fn the_corporate_deck_carries_its_elements() {
    let bytes = mjx_fixtures::fixture("corporate.pptx");
    let mut deck = Presentation::open(&bytes).expect("the corporate deck opens");

    assert_eq!(
        deck.slide_count(),
        1,
        "the corporate deck is one slide, so every assertion below is about the page the \
         acceptance render draws"
    );
    assert!(
        deck.master_count() >= 1,
        "the deck states no master, so the gradient background and the logo have nowhere to live"
    );
    assert!(
        deck.theme(Surface::Master(0))
            .expect("the theme part reads")
            .is_some(),
        "the deck's master reaches no theme part, so a chart's series colours resolve to nothing"
    );
    assert!(
        deck.theme_accent_colors(Surface::Slide(0))
            .expect("the accents read")
            .is_some(),
        "the theme states no accent colours, so `a chart using theme colours` cannot be what the \
         chart uses"
    );

    // A logo picture on the master: a picture shape whose image relationship resolves.
    let master_shapes = deck.shape_count(Surface::Master(0)).unwrap_or(0);
    let master_pictures = (0..master_shapes)
        .filter(|shape| {
            deck.picture_image_rel_id(Surface::Master(0), *shape)
                .ok()
                .flatten()
                .is_some()
        })
        .count();
    assert!(
        master_pictures >= 1,
        "the master carries {master_pictures} picture(s); MJXOFF-300 lists a logo picture on it"
    );

    let slide_shapes = deck.shape_count(Surface::Slide(0)).unwrap_or(0);
    assert!(
        slide_shapes >= 6,
        "the corporate slide holds {slide_shapes} shape(s). The ticket lists a title, two pictures, \
         a custom-geometry icon, a connector, a chart, a diagram, a table, an overlay and a list — \
         a slide this sparse is not the fixture RC03 asks for."
    );

    // A custom geometry, through the typed reader.
    let custom = (0..slide_shapes)
        .filter(|shape| {
            matches!(
                deck.shape_geometry(Surface::Slide(0), *shape),
                Ok(Geometry::Custom(_))
            )
        })
        .count();
    assert!(
        custom >= 1,
        "no shape on the slide answers `Geometry::Custom`; MJXOFF-300 lists a `custGeom` icon"
    );

    // The graphic frames: a table, a chart and a SmartArt diagram, through the typed reader.
    let mut frames = BTreeSet::new();
    for shape in 0..slide_shapes {
        if let Ok(Some(kind)) = deck.graphic_frame_kind(Surface::Slide(0), shape) {
            frames.insert(format!("{kind:?}"));
        }
    }
    for wanted in ["Table", "Chart", "Diagram"] {
        assert!(
            frames.contains(wanted),
            "the slide frames no `{wanted}`; MJXOFF-300 lists a styled table, a chart and a \
             SmartArt frame. The frames it does hold: {frames:?}"
        );
    }

    // The table is styled, and it is a real grid rather than a single cell.
    let table = (0..slide_shapes)
        .find(|shape| {
            matches!(
                deck.graphic_frame_kind(Surface::Slide(0), *shape),
                Ok(Some(mjx_pptx::GraphicFrameKind::Table))
            )
        })
        .expect("the table frame found above");
    assert!(
        deck.table_style_id(Surface::Slide(0), table)
            .expect("the table reads")
            .is_some(),
        "the table names no style, so `a styled table` is an unstyled one"
    );
    let (rows, columns) = deck
        .table_dimensions(Surface::Slide(0), table)
        .expect("the table reads");
    assert!(
        rows >= 2 && columns >= 2,
        "the table is {rows}x{columns}; a banded style cannot show on a grid that small"
    );

    // The chart part is really there, and so is the diagram's.
    assert!(
        deck.chart_part_bytes(Surface::Slide(0), 0).is_ok(),
        "the chart frame reaches no chart part"
    );

    // The slide-number and date fields, through the typed reader.
    let mut fields = BTreeSet::new();
    for shape in 0..slide_shapes {
        let paragraphs = deck.paragraph_count(Surface::Slide(0), shape).unwrap_or(0);
        for paragraph in 0..paragraphs {
            let count = deck
                .paragraph_field_count(Surface::Slide(0), shape, paragraph)
                .unwrap_or(0);
            for field in 0..count {
                if let Ok(Some(kind)) =
                    deck.paragraph_field_type(Surface::Slide(0), shape, paragraph, field)
                {
                    fields.insert(kind);
                }
            }
        }
    }
    for wanted in ["slidenum", "datetime"] {
        assert!(
            fields.iter().any(|kind| kind.starts_with(wanted)),
            "the slide states no `{wanted}` field; MJXOFF-300 lists slide-number and date fields. \
             The fields it does state: {fields:?}"
        );
    }

    assert_probes("corporate.pptx", DECK_PROBES);
}

/// The workbook carries the elements MJXOFF-300 lists, through the typed model where one exists.
#[test]
fn the_corporate_workbook_carries_its_elements() {
    let bytes = mjx_fixtures::fixture("corporate.xlsx");
    let mut book = Workbook::open(&bytes).expect("the corporate workbook opens");

    assert!(
        book.theme_colors().expect("the theme part reads").is_some(),
        "the workbook reaches no theme part, so a `<color theme=\"4\"/>` resolves to nothing"
    );

    // The styled table, through the typed reader, with the style the ticket names.
    let tables = book.sheet_tables(0).expect("the tables read");
    assert_eq!(
        tables.len(),
        1,
        "sheet 0 states {} table(s); MJXOFF-300 lists one worksheet table",
        tables.len()
    );
    assert_eq!(
        tables[0].style_name.as_deref(),
        Some("TableStyleMedium2"),
        "the worksheet table names style {:?}; MJXOFF-300 names `TableStyleMedium2`",
        tables[0].style_name
    );
    assert_eq!(
        tables[0].header_row_count, 1,
        "the table states {} header row(s); a banded table style shows on a header row",
        tables[0].header_row_count
    );

    // The drawing: a picture and a text-box shape, through the typed reader.
    let drawing = book
        .sheet_drawing(0)
        .expect("the drawing part reads")
        .expect("sheet 0 reaches a drawing part; MJXOFF-300 lists a picture and a text box on it");
    let pictures = drawing
        .objects
        .iter()
        .filter(|object| object.image.is_some())
        .count();
    assert!(
        pictures >= 1,
        "the sheet's drawing anchors {pictures} picture(s) whose image part resolves"
    );
    let shapes = drawing
        .objects
        .iter()
        .filter(|object| object.object == Some("sp"))
        .count();
    assert!(
        shapes >= 1,
        "the sheet's drawing anchors {shapes} `xdr:sp`; MJXOFF-300 lists a text-box shape"
    );

    // The chart part is really there.
    assert!(
        book.chart_part_bytes(0, 0).is_ok(),
        "the workbook reaches no chart part"
    );

    assert_probes("corporate.xlsx", SHEET_PROBES);
}

/// The document carries the elements MJXOFF-300 lists, through the typed model where one exists.
#[test]
fn the_corporate_document_carries_its_elements() {
    let bytes = mjx_fixtures::fixture("corporate.docx");
    let mut document = Document::open(&bytes).expect("the corporate document opens");

    // Tracked changes, through the typed reader — the one element of the three formats' lists with
    // a first-class accessor.
    let revisions = document.revisions().expect("the revisions read");
    assert!(
        !revisions.is_empty(),
        "the document records no tracked change; MJXOFF-300 lists them"
    );

    // A footnotes part that really exists.
    assert!(
        document
            .footnotes(|footnotes, interner| footnotes.user_footnotes(interner).count())
            .expect("the footnotes part reads")
            .is_some_and(|count| count >= 1),
        "the document reaches no footnote of its own; MJXOFF-300 lists footnotes"
    );

    // The style sheet, and a header the section really resolves.
    let styles = document
        .style_sheet(|sheet, _| sheet.style_count())
        .expect("the styles part reads")
        .expect("the document reaches a styles part");
    assert!(
        styles >= 4,
        "the document defines {styles} style(s); MJXOFF-300 lists styles carrying theme colours"
    );
    assert!(
        document
            .resolve_header(0, mjx_docx::HeaderFooterType::Default)
            .expect("the header resolves")
            .is_some(),
        "section 0 shows no default header; MJXOFF-300 lists a header with a three-cell table and a \
         logo"
    );

    // The equations and the drawings, through the read-once residency the box model consumes.
    // A drawing and an equation belong to the paragraph that carries them, so the document's count
    // is the sum over its paragraphs rather than a field of its own.
    let formatting = document.formatting().expect("the document resolves");
    let equations: usize = formatting
        .paragraphs()
        .iter()
        .map(|paragraph| paragraph.equations().len())
        .sum();
    assert!(
        equations >= 1,
        "the residency reports {equations} equation(s); MJXOFF-300 lists one"
    );
    let drawings: usize = formatting
        .paragraphs()
        .iter()
        .map(|paragraph| paragraph.drawings().len())
        .sum();
    assert!(
        drawings >= 2,
        "the residency reports {drawings} drawing(s); MJXOFF-300 lists an inline picture, a \
         floating picture and a text box"
    );
    assert!(
        !formatting.sections().is_empty(),
        "the document states no section, so the acceptance render has no page geometry to lay out \
         against"
    );

    assert_probes("corporate.docx", DOCUMENT_PROBES);
}

// ---------------------------------------------------------------------------------------------
// What RC03's audit found claimed and unexercised (MJXOFF-300)
//
// Four of the coverage claims above were true of the README and not of the file: every `a:avLst`
// was empty, the inherited placeholders held empty runs, the cached drawing was an empty
// `dsp:spTree`, and the crop was spliced before *the first* `a:stretch` on the slide rather than
// onto a picture. Each is now an element, and each element has an assertion here.
// ---------------------------------------------------------------------------------------------

/// What the blank master states in `p:titleStyle` (`sz="4400"`) — the size a title placeholder that
/// states none of its own is laid out at. Sourced from `crates/mjx-pptx/src/blank.rs`.
const MASTER_TITLE_POINTS: f64 = 44.0;

/// What that master states in `p:bodyStyle`'s first outline level (`sz="2800"`), on the same terms.
const MASTER_BODY_POINTS: f64 = 28.0;

/// The corner radius the overlay's `roundRect` states, as a fraction of its shorter side.
const OVERLAY_CORNER_RADIUS: f64 = 0.25;

/// One part of a fixture, as text.
fn part_markup(fixture: &str, part: &str) -> String {
    let bytes = mjx_fixtures::fixture(fixture);
    let package = Package::open(&bytes).expect("the fixture is a well-formed package");
    let name = mjx_opc::PartName::new(part).expect("a part name");
    let payload = package
        .part_payload(&name)
        .unwrap_or_else(|| panic!("`{part}` is not in `{fixture}`"));
    String::from_utf8(payload.into_owned()).expect("the part is UTF-8")
}

/// Every `p:pic` element of a part, each as the text between its own tags.
fn picture_elements(markup: &str) -> Vec<&str> {
    let mut pictures = Vec::new();
    let mut rest = markup;
    while let Some(start) = rest.find("<p:pic") {
        let after = &rest[start..];
        let end = after
            .find("</p:pic>")
            .map_or(after.len(), |at| at + "</p:pic>".len());
        pictures.push(&after[..end]);
        rest = &after[end..];
    }
    pictures
}

/// **A preset on the slide really carries an overridden adjustment**, which is what
/// `pptx-shape-adjustments` claims and what an empty `a:avLst` does not exercise.
#[test]
fn a_preset_shapes_adjustment_is_overridden_and_the_production_reader_carries_it() {
    let bytes = mjx_fixtures::fixture("corporate.pptx");
    let mut deck = Presentation::open(&bytes).expect("the corporate deck opens");
    let count = deck
        .shape_count(Surface::Slide(0))
        .expect("the slide reads");

    let shape = (0..count)
        .find(|shape| {
            matches!(
                deck.shape_geometry(Surface::Slide(0), *shape),
                Ok(Geometry::Preset(ShapeGeometry::RoundedRectangle { .. }))
            )
        })
        .expect(
            "no shape on the slide is a `roundRect`. Every `a:prstGeom` here stated an empty \
             `a:avLst`, which is a shape with nothing to adjust — so the coverage claim on \
             `pptx-shape-adjustments` was about an element the fixture did not carry.",
        );

    let Ok(Geometry::Preset(ShapeGeometry::RoundedRectangle { corner_radius })) =
        deck.shape_geometry(Surface::Slide(0), shape)
    else {
        unreachable!("just matched")
    };
    assert!(
        (corner_radius.ratio() - OVERLAY_CORNER_RADIUS).abs() < 1e-9,
        "the typed reader saw a corner radius of {corner_radius:?}, not the one the fixture states"
    );

    let bounds = deck
        .effective_shape_bounds(Surface::Slide(0), shape)
        .expect("the shape's bounds read")
        .expect("the shape states its own bounds");
    let extents = Size::from_emu(bounds.width_emu, bounds.height_emu);
    let adjustments = deck
        .shape_adjustments(Surface::Slide(0), shape, GuideContext::from_size(extents))
        .expect("the adjustments resolve");
    let overridden: Vec<&str> = adjustments
        .iter()
        .filter(|adjustment| adjustment.is_overridden)
        .map(|adjustment| adjustment.spec.wire_name)
        .collect();
    assert_eq!(
        overridden,
        vec!["adj"],
        "the shape's `a:avLst` overrides {overridden:?}; a preset whose every adjustment is the \
         table's default exercises the adjustment path with nothing"
    );

    // And the production reader carries it across, which is what the provider actually resolves.
    let outline = shape_outline(
        &mut deck,
        Surface::Slide(0),
        &[u32::try_from(shape).expect("a small shape tree")],
        extents,
    )
    .expect("the production reader answers a preset shape");
    let carried: Vec<&str> = outline
        .adjustments
        .iter()
        .map(|adjustment| adjustment.wire_name.as_str())
        .collect();
    assert_eq!(
        carried,
        vec!["adj"],
        "`mjx_reference_pack::outlines::shape_outline` carried {carried:?} to the geometry \
         provider, so the override never reaches the outline that is drawn"
    );
}

/// **The inherited placeholders hold real text**, laid out at the size the master states.
///
/// `pptx-master-and-layout-shapes` and `pptx-placeholder-geometry` are claimed against these two
/// shapes, and until RC03's audit both held a single empty run: the master's title style was
/// exercised by no glyph.
#[test]
fn the_inherited_placeholders_carry_text_at_the_masters_own_size() {
    let bytes = mjx_fixtures::fixture("corporate.pptx");
    let mut deck = Presentation::open(&bytes).expect("the corporate deck opens");

    for (kind, points) in [
        (PlaceholderType::Title, MASTER_TITLE_POINTS),
        (PlaceholderType::Body, MASTER_BODY_POINTS),
    ] {
        let shape = deck
            .shape_for_placeholder(Surface::Slide(0), kind)
            .expect("the slide's shapes read")
            .unwrap_or_else(|| panic!("the slide offers no `{kind:?}` placeholder to fill"));
        let text = deck
            .shape_text(Surface::Slide(0), shape)
            .expect("the placeholder's text reads");
        assert!(
            !text.trim().is_empty(),
            "the `{kind:?}` placeholder holds no text, so the master's own text style is \
             exercised by nothing"
        );
        let effective = deck
            .effective_run_properties(Surface::Slide(0), shape, 0, 0)
            .expect("the run's effective properties resolve");
        assert_eq!(
            effective.size_points(),
            Some(points),
            "the `{kind:?}` placeholder's first run is laid out at {:?} rather than at the \
             {points} points the master states. The placeholder states no size of its own, so this \
             number can only have come from the master — which is the inheritance the two coverage \
             claims are about.",
            effective.size_points()
        );
    }
}

/// **The SmartArt frame's cached drawing holds real shapes**, which is the whole reason the
/// diagram-drawing namespace is on the preserved-foreign allowlist.
#[test]
fn the_smartart_cached_drawing_holds_real_shapes() {
    let bytes = mjx_fixtures::fixture("corporate.pptx");
    let mut deck = Presentation::open(&bytes).expect("the corporate deck opens");
    let count = deck
        .shape_count(Surface::Slide(0))
        .expect("the slide reads");
    let frame = (0..count)
        .find(|shape| {
            matches!(
                deck.graphic_frame_kind(Surface::Slide(0), *shape),
                Ok(Some(mjx_pptx::GraphicFrameKind::Diagram))
            )
        })
        .expect("the slide frames a SmartArt diagram");

    let parts = deck
        .diagram_parts(Surface::Slide(0), frame)
        .expect("the diagram's parts resolve")
        .expect("the frame names a diagram");
    let drawing = parts
        .drawing
        .expect("the diagram's data part reaches a cached drawing");
    let markup = String::from_utf8(
        deck.diagram_part_bytes(&drawing)
            .expect("the cached drawing's bytes")
            .into_owned(),
    )
    .expect("the cached drawing is UTF-8");

    // `<dsp:sp ` with the space: `<dsp:spTree>` and `<dsp:spPr>` are not shapes, and a needle that
    // counted them would have read an empty drawing as holding one.
    let shapes = markup.matches("<dsp:sp ").count();
    assert!(
        shapes >= 3,
        "the cached drawing holds {shapes} shape(s). An empty `dsp:spTree` is a drawing with \
         nothing cached in it, and it is the sole justification for the diagram-drawing namespace \
         on the schema gate's preserved-foreign allowlist."
    );
    for label in ["Plan", "Build", "Ship"] {
        assert!(
            markup.contains(&format!("<a:t>{label}</a:t>")),
            "the cached drawing carries no shape reading `{label}`, so its boxes are empty"
        );
    }
}

/// **The crop sits on the picture it was authored for**, and on no other.
///
/// The splice anchored on *the first `a:stretch` on the slide* until RC03's audit, so a change to
/// the order the writers emit pictures in would have moved the crop to the other picture with every
/// gate still green — the fixture would still have carried an `a:srcRect`, on the wrong shape.
#[test]
fn the_crop_sits_on_the_cropped_picture_and_on_no_other() {
    let markup = part_markup("corporate.pptx", "/ppt/slides/slide1.xml");
    let pictures = picture_elements(&markup);
    assert_eq!(
        pictures.len(),
        2,
        "the corporate slide holds {} `p:pic` shape(s); MJXOFF-300 lists a cropped one and an \
         ellipse-masked one",
        pictures.len()
    );

    let cropped: Vec<&&str> = pictures
        .iter()
        .filter(|picture| picture.contains("<a:srcRect"))
        .collect();
    assert_eq!(
        cropped.len(),
        1,
        "{} of the slide's pictures carry an `a:srcRect`",
        cropped.len()
    );
    // 0.6 in by 1.9 in, 3.0 in by 2.0 in — the cropped picture's own placement, which is what says
    // *which* picture this is without depending on the order the writers emit them in.
    assert!(
        cropped[0].contains(r#"<a:off x="548640" y="1737360"/>"#)
            && cropped[0].contains(r#"cx="2743200""#),
        "the `a:srcRect` sits on a picture at some other position, so the crop has moved to the \
         wrong shape: {}",
        cropped[0]
    );
    let masked = pictures
        .iter()
        .find(|picture| picture.contains(r#"x="3566160""#))
        .expect("the ellipse-masked picture sits at 3.9 in");
    assert!(
        !masked.contains("<a:srcRect"),
        "the ellipse-masked picture carries the crop as well"
    );
}
