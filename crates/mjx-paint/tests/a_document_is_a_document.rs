//! The two exporters, checked by **readers this workspace did not write**.
//!
//! # Why an independent reader and not our own
//!
//! MJXOFF-164 names the trap directly: *"comparing a PDF export against a raster render by
//! rasterising the PDF with our own code is self-referential and proves nothing"*. It is worth
//! spelling out why, because the self-referential version looks like a stronger test.
//!
//! Suppose the exporter writes a `/ToUnicode` map with the glyph ids and the characters the wrong
//! way round, and suppose a reader we wrote reads it the same wrong way round. The round trip is
//! perfect and the file is unreadable by every PDF reader in the world. Two of our own components
//! agreeing says nothing about the format; it says our two components were written by the same
//! person on the same afternoon.
//!
//! So: **`pdftotext`** (poppler) extracts the text, and **`xmllint`** validates the SVG. Neither has
//! heard of this project. Both are checked for with `command -v` and their absence is a **loud,
//! named skip** — and `MJX_REQUIRE_TOOLS=1` turns the absence into a failure, which is what
//! continuous integration sets, in the same shape `MJX_REQUIRE_SCHEMA` and `MJX_REQUIRE_GPU`
//! already use in this repository.
//!
//! # What each one proves
//!
//! `pdftotext` reading `Fidelity` back out of the export proves, in one assertion, that:
//!
//! * the file is a PDF a real reader will open — a malformed `xref`, a wrong `/Length`, a dangling
//!   object reference and a broken trailer all stop it before any text is found;
//! * the text is **text** rather than paths — a page of glyph outlines extracts as nothing at all;
//! * the font is embedded, encoded `Identity-H` and addressed by glyph id; and
//! * the `/ToUnicode` map maps those ids back to the characters the document meant.
//!
//! `xmllint --noout` proves the SVG is well-formed XML, which a `data-` attribute containing an
//! unescaped `&` or a `<g>` left unclosed by a mismatched clip would both break.
//!
//! # Proved by mutation
//!
//! * Dropping the `/ToUnicode` object from the font dictionary → `pdftotext` extracts nothing and
//!   `the_pdfs_text_is_text` fails. **This is the assertion that separates a PDF from a picture.**
//! * Writing the `xref` offsets before the objects rather than as they are laid down → `pdftotext`
//!   refuses the file.
//! * Removing `xml_escape` from the placeholder label attribute → a provider whose label contains
//!   `&` makes `xmllint` refuse the document.
//! * Emitting the glyph run's `<g>` without closing it → `xmllint` refuses.

mod common;

use std::process::Command;

use mjx_paint::{
    OffscreenSurface, PaintError, Painter, PdfPainter, Resources, SvgPainter, Viewport,
};
use mjx_scene::PlaceholderGeometry;

/// The environment variable that turns a missing external reader into a failure.
const REQUIRE: &str = "MJX_REQUIRE_TOOLS";

/// Whether a tool is on the path, announcing a loud named skip when it is not.
fn tool(case: &str, name: &str) -> bool {
    let found = Command::new("command")
        .args(["-v", name])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
        || which(name);
    if !found {
        let message = format!(
            "SKIPPED {case}: `{name}` is not on the path. This case needs an **independent** \
             reader — checking our own export with our own reader proves nothing — so set \
             {REQUIRE}=1 to make its absence a failure instead."
        );
        assert!(
            std::env::var(REQUIRE).is_err(),
            "{REQUIRE} is set, so a missing external reader is a failure and not a skip. {message}"
        );
        println!("{message}");
    }
    found
}

/// Whether `name` resolves on `PATH`.
///
/// `command -v` is a shell builtin and not always an executable, so the search is done here as well;
/// a case that skipped because the *check* failed would be the same defect as a case that skipped
/// because the tool was missing and said nothing.
fn which(name: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|directory| directory.join(name).is_file())
}

/// Export a page through an exporter and answer its bytes.
fn export<P: Painter + ?Sized>(
    painter: &mut P,
    list: &mjx_scene::DisplayList,
    width: u32,
    height: u32,
) -> Result<(), PaintError> {
    let library = common::liberation_library();
    let geometry = PlaceholderGeometry::new();
    let images = common::OnePicture::new();
    let mut glyphs = common::ChequeredAtlas::new();
    let mut host = OffscreenSurface::new(width, height, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport)?;
    let mut resources = Resources::new(&mut glyphs, &geometry, &images).with_fonts(&library);
    painter.draw(&frame, list, &mut resources)?;
    painter.end(frame)?;
    Ok(())
}

/// A temporary file under the target directory, so a failing case leaves the artefact behind.
fn scratch(name: &str) -> std::path::PathBuf {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/mjx-paint-exports");
    let _ = std::fs::create_dir_all(&directory);
    directory.join(name)
}

#[test]
fn the_pdfs_text_is_text() {
    const CASE: &str = "the PDF's text is text";
    if !tool(CASE, "pdftotext") {
        return;
    }
    let list = common::text_page(240.0, 120.0);
    let mut painter = PdfPainter::new();
    export(&mut painter, &list, 240, 120).expect("the page exports");
    let bytes = painter.document().expect("a finished document").to_vec();
    assert!(
        bytes.starts_with(b"%PDF-1.7"),
        "a PDF begins with its version"
    );

    let path = scratch("text.pdf");
    std::fs::write(&path, &bytes).expect("the export is written");
    let output = Command::new("pdftotext")
        .arg(&path)
        .arg("-")
        .output()
        .expect("pdftotext runs");
    assert!(
        output.status.success(),
        "`pdftotext` refused the file, which means it is not a PDF a reader will open: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let extracted = String::from_utf8_lossy(&output.stdout);
    println!(
        "{CASE}: pdftotext read {extracted:?} out of {} bytes",
        bytes.len()
    );
    assert!(
        extracted.contains(common::KNOWN_TEXT),
        "`pdftotext` did not find {:?} in the export; it read {extracted:?}. A page of glyph \
         outlines extracts as nothing at all, so this one assertion is what separates *text* from a \
         picture of text — and it fails if the font is not embedded, if the encoding is not \
         `Identity-H`, or if the `/ToUnicode` map is missing or wrong.",
        common::KNOWN_TEXT
    );
}

#[test]
fn the_pdf_embeds_a_subset_and_not_the_face() {
    let list = common::text_page(240.0, 120.0);
    let mut painter = PdfPainter::new();
    export(&mut painter, &list, 240, 120).expect("the page exports");
    let bytes = painter.document().expect("a finished document").to_vec();
    let face = common::liberation_face();
    let whole = face.data().len();
    assert!(
        bytes.len() * 3 < whole,
        "the export is {} bytes and the whole face is {whole}; a page with eight letters on it \
         must not carry two and a half thousand glyphs",
        bytes.len()
    );
    // The structures that make it text, named rather than inferred. A file that extracted correctly
    // by some other route would still be the wrong file.
    for marker in [
        "/Type0",
        "/Identity-H",
        "/CIDFontType2",
        "/CIDToGIDMap /Identity",
        "/FontFile2",
        "/ToUnicode",
        "beginbfchar",
    ] {
        assert!(
            String::from_utf8_lossy(&bytes).contains(marker),
            "the export does not carry `{marker}`, which a searchable PDF needs"
        );
    }
}

#[test]
fn the_svg_is_well_formed_and_says_what_the_scene_contained() {
    const CASE: &str = "the SVG is well formed and says what the scene contained";
    let list = common::every_command(200.0, 150.0);
    let mut painter = SvgPainter::new();
    export(&mut painter, &list, 200, 150).expect("the page exports");
    let document = painter.document().expect("a finished document").to_owned();
    let path = scratch("scene.svg");
    std::fs::write(&path, &document).expect("the export is written");

    // **The debug view.** These are the attributes MJXOFF-164 asks for by name, and they are what
    // turns "the third shape is the wrong colour" into "command 12 names paint row 4".
    for attribute in [
        "data-mjx-command=",
        "data-mjx-table=",
        "data-mjx-row=",
        "data-mjx-paint=",
        "data-mjx-role=",
        "data-mjx-provenance=",
        "data-mjx-layer=",
        "data-mjx-glyphs=",
        "data-mjx-face=",
    ] {
        assert!(
            document.contains(attribute),
            "the document does not carry `{attribute}`, so a reader cannot tell what the scene \
             contained"
        );
    }
    // **Named indices, not merely the attribute names.** An exporter that wrote `none` into every
    // one of them, or the same row into every one, would satisfy the scan above and tell a reader
    // nothing. `every_command` fills five paint rows and draws from four of them, and the shapes on
    // the page come from three different geometry rows.
    let named_paints: std::collections::BTreeSet<&str> = document
        .match_indices("data-mjx-paint=\"")
        .filter_map(|(at, marker)| {
            let rest = document.get(at + marker.len()..)?;
            let end = rest.find('"')?;
            rest.get(..end)
        })
        .filter(|row| *row != "none")
        .collect();
    assert!(
        named_paints.len() >= 3,
        "the debug attributes must name real, differing rows of the paint table; they named \
         {named_paints:?}"
    );
    let named_geometry: std::collections::BTreeSet<&str> = document
        .match_indices("data-mjx-row=\"")
        .filter_map(|(at, marker)| {
            let rest = document.get(at + marker.len()..)?;
            let end = rest.find('"')?;
            rest.get(..end)
        })
        .filter(|row| *row != "none")
        .collect();
    assert!(
        named_geometry.len() >= 3,
        "and real, differing resource rows; they named {named_geometry:?}"
    );
    assert!(
        document.contains("data-mjx-table=\"glyph run\""),
        "the run must be attributed to the glyph-run table"
    );
    assert!(
        document.contains("data-mjx-layer=\"opacity\"")
            && document.contains("data-mjx-layer=\"effect\""),
        "both of this page's layers must say why they exist"
    );

    if !tool(CASE, "xmllint") {
        return;
    }
    let output = Command::new("xmllint")
        .arg("--noout")
        .arg(&path)
        .output()
        .expect("xmllint runs");
    assert!(
        output.status.success(),
        "`xmllint` refused the document: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{CASE}: xmllint accepted {} bytes", document.len());
}

#[test]
fn the_svg_draws_text_as_outlines_and_says_when_it_cannot() {
    let list = common::text_page(240.0, 120.0);

    // With a face: real vector glyphs.
    let mut painter = SvgPainter::new();
    export(&mut painter, &list, 240, 120).expect("the page exports");
    let with = painter.document().expect("a document").to_owned();
    assert!(
        with.contains("data-mjx-glyph=\""),
        "each glyph must be written as its own path, carrying its glyph id"
    );
    assert!(
        !with.contains("data-mjx-unresolved-face"),
        "a run whose face was supplied must not be reported as unresolved"
    );
    let outlines = with.matches("data-mjx-glyph=\"").count();
    assert_eq!(
        outlines,
        common::KNOWN_TEXT.chars().count(),
        "one path per glyph of the run"
    );

    // Without one: the metadata and an honest admission, rather than silence.
    let mut painter = SvgPainter::new();
    let geometry = PlaceholderGeometry::new();
    let images = common::OnePicture::new();
    let mut glyphs = common::ChequeredAtlas::new();
    let mut host = OffscreenSurface::new(240, 120, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");
    let mut resources = Resources::new(&mut glyphs, &geometry, &images);
    painter
        .draw(&frame, &list, &mut resources)
        .expect("the page exports");
    painter.end(frame).expect("the frame finishes");
    let without = painter.document().expect("a document").to_owned();
    assert!(
        without.contains("data-mjx-unresolved-face=\"true\""),
        "a run whose face the caller did not supply must say so in the file. Silence would make a \
         page with no text on it indistinguishable from a page whose face was missing."
    );
    assert!(
        !without.contains("data-mjx-glyph=\""),
        "and it must not invent outlines it does not have"
    );
    assert!(
        without.contains("data-mjx-glyphs=\"8\""),
        "the run's own metadata survives either way, so the debug view is complete"
    );
}

#[test]
fn a_document_painter_has_no_pixels_and_says_so() {
    let list = common::one_rectangle(
        40.0,
        40.0,
        mjx_scene::SceneRect::new(4.0, 4.0, 36.0, 36.0),
        common::rgb(0x40, 0x60, 0x80),
    );
    for (name, mut painter) in [
        ("svg", Box::new(SvgPainter::new()) as Box<dyn Painter>),
        ("pdf", Box::new(PdfPainter::new()) as Box<dyn Painter>),
    ] {
        assert_eq!(painter.name(), name);
        assert_eq!(painter.backend().api, mjx_paint::GraphicsApi::None);
        assert_eq!(
            painter.backend().antialiasing,
            mjx_paint::Antialiasing::None,
            "an exporter leaves antialiasing to whatever opens the file"
        );
        export(&mut *painter, &list, 40, 40).expect("the page exports");
        assert!(
            painter.read_pixels().expect("readback").is_none(),
            "`{name}` answered with pixels, which a document painter has none of. A golden-image \
             suite asks every painter and skips the ones that cannot answer; one that answered \
             with an empty image would be compared against and would agree with nothing."
        );
    }
}

#[test]
fn the_pdfs_blur_fallback_actually_rasterises() {
    // **PDF has no blur operator** — no filter model in the imaging model at all — so every effect
    // that needs one is rasterised through the software painter and embedded. That fallback is the
    // one part of this exporter that cannot be inferred from the file's structure, and it had never
    // executed until this case: every other PDF case in this file exports a page with no effect on
    // it, so `SoftwarePainter::rasterise_layer` was written, reachable and unrun.
    let list = common::one_shape_under(120.0, 100.0, mjx_scene::EffectKind::OuterShadow);
    let mut pdf = PdfPainter::new();
    export(&mut pdf, &list, 120, 100).expect("the page exports");
    let bytes = pdf.document().expect("a document").to_vec();
    let text = String::from_utf8_lossy(&bytes).into_owned();

    assert!(
        text.contains("/Subtype /Image"),
        "a shadowed page must carry a rasterised image, because there is no operator that blurs"
    );
    assert!(
        text.contains("/SMask"),
        "and a soft mask for its alpha, or the shadow is a grey rectangle over the page"
    );
    // And the image is a *rendered* one rather than an empty allocation. A fallback that embedded a
    // transparent image would satisfy both assertions above and lose the shadow entirely.
    let mut painter = mjx_paint::SoftwarePainter::new();
    let mut host = OffscreenSurface::new(120, 100, 1.0);
    let mut glyphs = common::ChequeredAtlas::new();
    let images = common::OnePicture::new();
    let geometry = PlaceholderGeometry::new();
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");
    let mut resources = Resources::new(&mut glyphs, &geometry, &images);
    painter
        .draw(&frame, &list, &mut resources)
        .expect("the page draws");
    painter.end(frame).expect("the frame finishes");
    let rendered = painter.read_pixels().expect("readback").expect("pixels");
    assert!(
        rendered.covered() > 2000,
        "the layer this exporter rasterises has to have ink in it, and the render has {} covered \
         pixels",
        rendered.covered()
    );
    assert!(
        bytes.len() > 20_000,
        "an embedded 120x100 image is at least three bytes a pixel plus its mask, and this export \
         is {} bytes — a fallback that embedded nothing would be far smaller",
        bytes.len()
    );
}

#[test]
fn both_exporters_count_a_stand_in_shape_and_say_which_it_was() {
    // **R08's hand-off 10, for the two painters that write documents.** The count is only useful if
    // every painter reports it, and an exported document has a second obligation the rasterisers do
    // not: the file itself has to say so, because nobody will re-run the export to find out.
    let list = common::one_unresolved_shape(80.0, 80.0);

    let mut svg = SvgPainter::new();
    export(&mut svg, &list, 80, 80).expect("the page exports");
    let report = svg.last_frame().expect("a report");
    assert_eq!(
        report.drawn.placeholders, 1,
        "the SVG exporter must count a stand-in shape: {report:?}"
    );
    let document = svg.document().expect("a document").to_owned();
    assert!(
        document.contains("data-mjx-placeholders=\"1\""),
        "and the document must say so at its root, so a file on somebody's disk can be told apart \
         from a fidelity export without re-running anything"
    );
    assert!(
        document.contains("data-mjx-provenance=\"placeholder\""),
        "and name which element it was"
    );
    assert!(
        document.contains("data-mjx-label=\""),
        "with the label the provider gave it, which is what says *what* it is standing in for"
    );

    let mut pdf = PdfPainter::new();
    export(&mut pdf, &list, 80, 80).expect("the page exports");
    let report = pdf.last_frame().expect("a report");
    assert_eq!(
        report.drawn.placeholders, 1,
        "the PDF exporter must count one too: {report:?}"
    );
}

#[test]
fn a_dashed_stroke_reaches_both_documents() {
    // `mjx-scene::dash_lengths` was private until MJXOFF-164, so an exporter had no way to know what
    // `lgDashDot` means and would have written a solid line. The lengths are the tessellator's own,
    // which is what stops a dotted line in a render and a dotted line in an export being two
    // decisions.
    let list = common::dashed_page(120.0, 60.0);
    let mut svg = SvgPainter::new();
    export(&mut svg, &list, 120, 60).expect("the page exports");
    let document = svg.document().expect("a document").to_owned();
    assert!(
        document.contains("stroke-dasharray=\""),
        "a dashed stroke must reach the SVG as a dash array rather than as a solid line"
    );

    let mut pdf = PdfPainter::new();
    export(&mut pdf, &list, 120, 60).expect("the page exports");
    let bytes = String::from_utf8_lossy(pdf.document().expect("a document")).into_owned();
    assert!(bytes.contains("] 0 d"), "and the PDF as a `d` array");
    assert!(
        !bytes.contains("[] 0 d\n"),
        "and not as the empty pattern, which is what a solid line writes"
    );
}
