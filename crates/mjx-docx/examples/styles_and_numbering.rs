//! Authoring `word/styles.xml` and `word/numbering.xml` into a document that has neither, and
//! reading the ladder back — the runnable version of
//! [the styles-and-inheritance guide](mjx_docx::guide::styles_and_inheritance).
//!
//! ```sh
//! cargo run -p mjx-docx --example styles_and_numbering -- out.docx
//! ```
//!
//! The assertions are on *effective* properties, not on the markup that was written. A run that
//! states nothing of its own still renders at some size, and that answer comes from a ladder —
//! document defaults, then the numbering level, then the paragraph-style chain, then the character
//! style, then direct formatting. Asserting that `effective_run_properties` moved when a rung moved
//! is the only check that distinguishes "the element was written" from "the element is consulted".

use anyhow::{Context, Result};
use mjx_docx::{
    AbstractNumbering, Document, LevelNumberFormat, LevelTextTemplate, NumberingInstance,
    NumberingLevel, PageSize, StyleDefinition,
};
use mjx_ooxml_types::wordprocessingml::{HalfPointMeasure, NumberFormat, StyleType};

mod support;

/// The numbering instance this example attaches its list paragraphs to.
const NUMBERING_ID: i64 = 1;
/// The abstract numbering definition that instance points at.
const ABSTRACT_NUMBERING_ID: i64 = 1;

fn main() -> Result<()> {
    let out = support::output_path("styles_and_numbering.docx");
    let mut document = Document::blank(PageSize::a4()).context("blank document")?;

    // ---- A blank document has no styles at all ------------------------------------------------
    // `style_sheet` answers `None` — the closure is never called — when there is no
    // `word/styles.xml`, which is exactly a blank document's state. Nothing is fabricated.
    anyhow::ensure!(
        document.style_sheet(|_, _| ())?.is_none(),
        "a blank document should relate to no word/styles.xml"
    );
    document.insert_run(0, 0, "A heading")?;
    let baseline = document.effective_run_properties(0, 0)?;
    anyhow::ensure!(
        baseline.font_size.is_none() && baseline.bold.is_none(),
        "with no styles.xml at all, every rung of the ladder is silent"
    );

    // ---- Document defaults: the bottom rung ------------------------------------------------------
    // `edit_style_sheet` creates `word/styles.xml`, registers its content type and relates it from
    // the main document part, all on the first call. Ten point, for every run that says nothing.
    document.edit_style_sheet(|sheet, interner| {
        let defaults = sheet.document_defaults_or_insert(interner);
        let run_defaults = defaults.run_properties_default_or_insert(interner);
        run_defaults
            .run_properties_or_insert(interner)
            .set_font_size(interner, Some(HalfPointMeasure::from_wire("20")));
    })?;
    let with_defaults = document.effective_run_properties(0, 0)?;
    println!(
        "effective font size from docDefaults: {:?}",
        with_defaults.font_size
    );
    anyhow::ensure!(
        with_defaults.font_size == Some(HalfPointMeasure::from_wire("20")),
        "the document default should now be the effective answer"
    );

    // ---- A style, and the chain above the defaults ---------------------------------------------
    // `Heading1` is fourteen point and bold. It is written but not yet *referenced* by anything, so
    // the effective answer for paragraph 0 must not move — a style nobody names changes nothing, and
    // an assertion that skipped this step could not tell a consulted style from an ignored one.
    document.edit_style_sheet(|sheet, interner| {
        let mut style = StyleDefinition::new(interner, StyleType::Paragraph, "Heading1");
        style.set_name(interner, Some("heading 1"));
        let properties = style.run_properties_or_insert(interner);
        properties.set_font_size(interner, Some(HalfPointMeasure::from_wire("28")));
        properties.set_bold(interner, Some(true));
        sheet.add_style(style);
    })?;
    let unreferenced = document.effective_run_properties(0, 0)?;
    anyhow::ensure!(
        unreferenced.font_size == Some(HalfPointMeasure::from_wire("20"))
            && unreferenced.bold.is_none(),
        "a style nothing references must not change the effective answer"
    );

    let style_count = document
        .style_sheet(|sheet, _| sheet.style_count())?
        .context("styles.xml exists by now")?;
    println!("styles.xml: {style_count} style(s)");

    // ---- Numbering: a definition, an instance, and a paragraph attached to it ---------------------
    // Two elements, two identifiers. `w:abstractNum` is the definition — the formats, the templates,
    // the start values. `w:num` is the instance a paragraph actually names, and it points at the
    // definition. `attach_paragraph_to_list` writes the paragraph's own `w:numPr`.
    document.edit_numbering(|numbering, interner| {
        let mut level = NumberingLevel::new(interner, 0);
        level.set_start(interner, Some(1));
        level.set_format(Some(LevelNumberFormat::new(
            interner,
            NumberFormat::Decimal,
        )));
        level.set_text_template(Some(LevelTextTemplate::new(interner, "%1.")));
        // A numbering level carries run properties of its own, one rung above the document
        // defaults and one below the paragraph-style chain: twelve point for list text.
        let mut level_runs = mjx_docx::RunProperties::new(interner);
        level_runs.set_font_size(interner, Some(HalfPointMeasure::from_wire("24")));
        level.set_run_properties(Some(level_runs));

        let mut definition = AbstractNumbering::new(interner, ABSTRACT_NUMBERING_ID);
        definition.push_level(level);
        numbering.push_abstract_numbering(definition);
        numbering.push_instance(NumberingInstance::new(
            interner,
            NUMBERING_ID,
            ABSTRACT_NUMBERING_ID,
        ));
    })?;

    document.append_paragraph()?;
    document.append_run(1, "First list item")?;
    document.attach_paragraph_to_list(1, NUMBERING_ID, 0)?;
    document.append_paragraph()?;
    document.append_run(2, "Second list item")?;
    document.attach_paragraph_to_list(2, NUMBERING_ID, 0)?;

    // The numbering rung now answers for the list paragraphs, and only for them: paragraph 0 is not
    // attached to a list, so it still reads the document default. One property, two answers,
    // decided by which rung is present — which is what makes this check discriminating.
    let list_run = document.effective_run_properties(1, 0)?;
    let plain_run = document.effective_run_properties(0, 0)?;
    println!(
        "list paragraph: {:?}; unattached paragraph: {:?}",
        list_run.font_size, plain_run.font_size
    );
    anyhow::ensure!(
        list_run.font_size == Some(HalfPointMeasure::from_wire("24")),
        "the numbering level should outrank the document default"
    );
    anyhow::ensure!(
        plain_run.font_size == Some(HalfPointMeasure::from_wire("20")),
        "a paragraph attached to no list must still read the document default"
    );

    // `resolve_numbering` walks instance → definition → level, following `w:numStyleLink`
    // indirection when a definition uses one.
    let start = document.resolve_numbering(NUMBERING_ID, 0, |lookup, _| match lookup {
        mjx_docx::NumberingLookup::Resolved(resolution) => resolution.effective_start(),
        mjx_docx::NumberingLookup::None => None,
    })?;
    println!("numbering {NUMBERING_ID} level 0 starts at {start:?}");
    anyhow::ensure!(start == Some(1), "the level's own w:start should resolve");

    // ---- Save, reopen, and read the whole ladder back ----------------------------------------------
    let bytes = document.save().context("saving")?;
    std::fs::write(&out, &bytes).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), bytes.len());

    let mut reopened = Document::open(&bytes).context("reopening")?;
    anyhow::ensure!(
        reopened.parts().styles.is_some() && reopened.parts().numbering.is_some(),
        "both authored parts should be related from the reopened document"
    );
    anyhow::ensure!(
        reopened.style_sheet(|sheet, _| sheet.style_count())? == Some(style_count),
        "the style sheet did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.effective_run_properties(1, 0)?.font_size
            == Some(HalfPointMeasure::from_wire("24")),
        "the numbering rung did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.effective_run_properties(0, 0)?.font_size
            == Some(HalfPointMeasure::from_wire("20")),
        "the document-defaults rung did not survive the round trip"
    );
    println!("reopened: the effective ladder answers the same on both sides of the round trip");

    Ok(())
}
