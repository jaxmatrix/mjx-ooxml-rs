//! The plate generator's output, **read back the way R11 and U01 will read it** — and the two
//! numbers a gallery is worthless without.
//!
//! # The question a hand-off has to answer
//!
//! Not *"is this produced?"* but *"does it reach anyone?"*. A manifest checked by asserting on the
//! string that produced it proves that `format!` works. So the manifest here is written to disk,
//! parsed back with [`mjx_render_oracle::json`], and every field a loader will look for is asked of
//! the parsed value — including the ones a loader needs in order to *refuse*: the schema version and
//! the premultiplication flag.
//!
//! # `placeholders`, and why it is asserted rather than labelled
//!
//! MJXOFF-165 was written while every preset shape in the workspace was a stand-in, and told this
//! child to say so in the gallery. **Phase G landed and it is no longer true**: all 186 published
//! preset geometries resolve and draw. A prose label would now be wrong, and would have been wrong
//! in the other direction before — so the gallery prints `DrawReport::placeholders`, measured, and
//! this file asserts it is zero.
//!
//! Zero is also true of a page that drew nothing at all, which is MJXOFF-206's finding one level up,
//! so `drawCalls` and `covered` are asserted beside it. Neither is sufficient alone.

use std::path::PathBuf;

use mjx_render_oracle::json::{parse, Value};
use mjx_render_oracle::plate::{generate, MANIFEST_FILE, MANIFEST_VERSION, NO_FORMAT_RENDERS_YET};
use mjx_render_oracle::specimen::{Perturbation, SPECIMENS};
use mjx_render_oracle::{gallery, specimen, Baselines, ReferenceProvider};

#[test]
fn every_plate_draws_the_document_and_not_a_stand_in() {
    for spec in SPECIMENS {
        let rendered = specimen::render(spec, Perturbation::None).expect("a render");
        assert_eq!(
            rendered.report.placeholders, 0,
            "`{}` drew {} stand-in shapes. Before Phase G every preset in this workspace was one; \
             a non-zero count now names a real hole — a handle nobody registered, a preset ECMA-376 \
             defines no geometry for, or a shape whose formulas are singular at these adjustments.",
            spec.name, rendered.report.placeholders
        );
        assert!(
            rendered.report.draw_calls > 0,
            "`{}` issued no draw calls, and zero placeholders is also true of a page that drew \
             nothing at all",
            spec.name
        );
        let image = mjx_render_oracle::png::image_from_pixels(&rendered.pixels);
        assert!(
            image.covered() > 500,
            "`{}` covered {} pixels, so it reported draws and drew nothing",
            spec.name,
            image.covered()
        );
        assert!(
            image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                > 2,
            "`{}` is at most two colours, which is a flat rectangle rather than a page",
            spec.name
        );
    }

    // And the counter is exercised on **both** of its two increments. `DrawReport::placeholders` is
    // incremented once under `FillPath` and once under `StrokePath`, and a corpus whose every shape
    // is filled proves one of them — which was true of this whole workspace until MJXOFF-206.
    let stroked = specimen::Specimen::named("preset-star").expect("the star");
    let list = specimen::display_list(&stroked, Perturbation::None).expect("a display list");
    let strokes = list
        .commands()
        .filter(|command| matches!(command, mjx_scene::Command::StrokePath { .. }))
        .count();
    assert!(
        strokes > 0,
        "no specimen strokes an outline, so half of the placeholder counter is unexercised"
    );
}

#[test]
fn the_manifest_answers_every_question_a_loader_will_ask() {
    let directory = scratch("manifest");
    let set = generate(ReferenceProvider::None, &Baselines::committed()).expect("plates generate");
    set.write(&directory).expect("the manifest writes");

    let text = std::fs::read_to_string(directory.join(MANIFEST_FILE)).expect("reading it back");
    let manifest = parse(&text).unwrap_or_else(|reason| {
        panic!("the manifest this crate wrote does not parse: {reason}\n{text}")
    });

    // The two fields a loader needs in order to **refuse**, which are the ones a generator is most
    // likely to leave out.
    assert_eq!(
        manifest.get("version").and_then(Value::number),
        Some(f64::from(MANIFEST_VERSION)),
        "a loader that cannot read the schema version cannot refuse a schema it does not know"
    );
    assert_eq!(
        manifest.get("premultiplied").and_then(Value::boolean),
        Some(false),
        "**the manifest does not state the alpha convention.** A consumer decoding these files has \
         to know, and the decision is easier to get wrong than to state."
    );
    assert_eq!(
        manifest.get("authoritative").and_then(Value::boolean),
        Some(false)
    );
    assert_eq!(
        manifest.get("parityClaimed").and_then(Value::boolean),
        Some(false)
    );
    assert!(manifest
        .get("generator")
        .and_then(Value::string)
        .is_some_and(|text| text.starts_with("mjx-render-oracle ")));

    let plates = manifest
        .get("plates")
        .and_then(Value::array)
        .expect("the manifest has a plate array");
    assert_eq!(plates.len(), SPECIMENS.len());
    for (plate, spec) in plates.iter().zip(SPECIMENS.iter()) {
        assert_eq!(plate.get("name").and_then(Value::string), Some(spec.name));

        // The file the loader will actually fetch, and it must be there.
        let file = plate
            .get("file")
            .and_then(Value::string)
            .expect("every plate names a file");
        let bytes = std::fs::read(directory.join(file)).expect("the plate's PNG is beside it");
        assert_eq!(
            plate.get("sha256").and_then(Value::string),
            Some(&*mjx_render_oracle::digest::sha256_hex(&bytes)),
            "`{}`'s recorded digest is not the digest of the file beside it, so a loader that \
             cached by it would serve the wrong image for ever",
            spec.name
        );

        // The dimensions a loader lays out with, checked against the file rather than against the
        // generator's own opinion.
        let image = mjx_render_oracle::png::decode(&bytes).expect("the plate decodes");
        assert_eq!(
            plate.get("width").and_then(Value::number),
            Some(f64::from(image.width))
        );
        assert_eq!(
            plate.get("height").and_then(Value::number),
            Some(f64::from(image.height))
        );

        // The four fields that stop a gallery being read as more than it is.
        assert_eq!(
            plate.get("placeholders").and_then(Value::number),
            Some(0.0),
            "`{}` reports stand-in geometry in the manifest",
            spec.name
        );
        assert!(plate
            .get("drawCalls")
            .and_then(Value::number)
            .is_some_and(|n| n > 0.0));
        assert!(plate
            .get("covered")
            .and_then(Value::number)
            .is_some_and(|n| n > 500.0));
        assert_eq!(plate.get("parity").and_then(Value::boolean), Some(false));
        assert_eq!(
            plate.get("reviewed").and_then(Value::boolean),
            Some(false),
            "`{}` claims a human review in the manifest",
            spec.name
        );
        assert!(plate
            .get("approver")
            .and_then(Value::string)
            .is_some_and(|text| text.contains("no human has looked")));
        assert!(plate.get("content").and_then(Value::string).is_some());
        assert!(plate.get("description").and_then(Value::string).is_some());
        // Under no reference at all, every plate carries a reason rather than a null.
        assert!(plate.get("excluded").and_then(Value::string).is_some());
    }

    // The document rows, derived from `mjx-fixtures` rather than from a list written here.
    let documents = manifest
        .get("documents")
        .and_then(Value::array)
        .expect("the manifest has a document array");
    assert_eq!(
        documents.len(),
        mjx_fixtures::package_fixtures().len(),
        "the gallery does not cover the committed corpus, so a fixture added tomorrow would be \
         missing from it and nothing would say so"
    );
    assert!(
        !documents.is_empty(),
        "the corpus is empty, which it is not"
    );
    for document in documents {
        assert!(document.get("fixture").and_then(Value::string).is_some());
        assert!(["pptx", "docx", "xlsx"]
            .contains(&document.get("format").and_then(Value::string).unwrap_or("")));
        assert_eq!(
            document.get("verdict").and_then(Value::string),
            Some("excluded"),
            "a document fixture claims a verdict, and no format renders yet"
        );
        assert_eq!(
            document.get("reason").and_then(Value::string),
            Some(NO_FORMAT_RENDERS_YET),
            "a row with no image and no reason looks like a failure rather than like a stage of the \
             project"
        );
    }

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn the_gallery_is_one_self_contained_file() {
    let directory = scratch("gallery");
    let set = generate(ReferenceProvider::None, &Baselines::committed()).expect("plates generate");
    set.write(&directory).expect("writing");
    let page = gallery::render(&set);
    std::fs::write(directory.join(gallery::GALLERY_FILE), &page).expect("writing the gallery");

    // No script, no stylesheet reference, no network: the page has to open from a downloaded
    // artefact directory on a machine with nothing installed, because a failure a reviewer cannot
    // see is a failure nobody fixes.
    for forbidden in ["<script", "http://", "https://", "<link "] {
        assert!(
            !page.contains(forbidden),
            "the gallery contains `{forbidden}`, so it does not open from an artefact directory"
        );
    }
    assert!(
        page.contains("<style>"),
        "the gallery has no styling at all"
    );

    // Every image it names is beside it.
    for plate in &set.plates {
        assert!(
            page.contains(&format!("src=\"{}\"", plate.file)),
            "the gallery does not show `{}`",
            plate.name
        );
        assert!(directory.join(&plate.file).is_file());
    }

    // The measured number, not a sentence about stand-ins.
    assert!(
        page.contains("Stand-in geometry: 0."),
        "the gallery does not print the measured placeholder count"
    );
    assert!(
        page.contains("Human review: 0 of 5."),
        "the gallery does not say how many images a person has looked at"
    );

    // And nothing a reader could mistake for a document rendering: the rows are there, with the
    // reason, and no image.
    assert!(page.contains("no format renders yet"));

    // Markup that came from a string is escaped, which matters because a plate's description and a
    // provider's label both reach the page as text.
    assert!(!page.contains("<script>alert"), "sanity");
    assert_eq!(
        gallery::escape("a & b < c > \"d\" 'e'"),
        "a &amp; b &lt; c &gt; &quot;d&quot; &#39;e&#39;"
    );

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn the_json_reader_is_strict_enough_to_be_worth_asserting_with() {
    // The reader is the instrument the manifest test measures with, so it needs its own instrument
    // test: a reader that quietly accepted anything would make every assertion above pass.
    assert!(parse("{").is_err());
    assert!(parse("{\"a\": }").is_err());
    assert!(parse("[1, 2").is_err());
    assert!(parse("{} trailing").is_err());
    assert!(parse("\"unterminated").is_err());
    assert!(parse("{\"a\": 1} ").is_ok(), "trailing whitespace is fine");

    let value = parse(
        "{\"n\": -1.5e2, \"t\": true, \"f\": false, \"z\": null, \
         \"s\": \"a\\\"b\\\\c\\nd\\u00e9\", \"a\": [1, {\"b\": 2}]}",
    )
    .expect("a document with every shape in it");
    assert_eq!(value.get("n").and_then(Value::number), Some(-150.0));
    assert_eq!(value.get("t").and_then(Value::boolean), Some(true));
    assert_eq!(value.get("f").and_then(Value::boolean), Some(false));
    assert_eq!(value.get("z"), Some(&Value::Null));
    assert_eq!(
        value.get("s").and_then(Value::string),
        Some("a\"b\\c\nd\u{e9}")
    );
    assert_eq!(
        value
            .get("a")
            .and_then(Value::array)
            .and_then(|items| items.get(1))
            .and_then(|item| item.get("b"))
            .and_then(Value::number),
        Some(2.0)
    );

    // And a string this crate writes survives being read back, escapes and multi-byte characters
    // included — the manifest carries prose with quotes and dashes in it.
    let awkward = "a \"quoted\" back\\slash, a newline\n, a tab\t and é — a dash";
    let round = parse(&format!(
        "{{\"x\": {}}}",
        mjx_render_oracle::json::quote(awkward)
    ))
    .expect("it parses");
    assert_eq!(round.get("x").and_then(Value::string), Some(awkward));
}

/// Where a case leaves its artefacts. Named per case, because Cargo runs a binary's cases on several
/// threads at once.
fn scratch(case: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/oracle-scratch")
        .join(case);
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}
