//! **The live token editor writes back, and changes nothing else.**
//!
//! MJXOFF-166's *Done when*: *"the live token editor writes back, proved by a round trip: change a
//! value in the harness, show it in the token source, regenerate, and show it in all three
//! generated artefacts."* The last two steps are `cargo run -p xtask -- tokens`, which this crate
//! may not call — nothing may depend on `xtask` — so the round trip is split where the layering
//! splits it:
//!
//! * **Here**: the write-back produces a source file that differs from the original in exactly one
//!   span, that span is the token's own `$value`, and the new value parses through the platform's
//!   own resolver. Both directions of a wrong write are refused — an unknown token and a malformed
//!   value.
//! * **`xtask/tests/tokens.rs`**: that the three artefacts are derived from the source, which is
//!   already a gate and did not need a second one.
//!
//! # ⚠ The assertion that matters is the one about what did *not* change
//!
//! A test that checked *"the new value is in the file"* would pass for an editor that rewrote the
//! whole file, reordered its keys, dropped its `$description` prose or reformatted every line — and
//! `tokens.json` is the one hand-edited artefact in the pipeline, whose reviewability is the reason
//! it is hand-edited. So [`the_write_back_changes_one_span_and_nothing_else`] compares the *whole
//! file outside the replaced span*, byte for byte.

use mjx_canvas_harness::tokens_source::{self, WriteBackError};
use mjx_tokens::Tokens;

/// The committed token source.
fn source() -> String {
    let path = tokens_source::source_path();
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

#[test]
fn the_write_back_changes_one_span_and_nothing_else() {
    let before = source();
    // A token the canvas actually reads, so a person who changes it in the harness sees sixty-one
    // images move: `document.light.selection-handle` is what every selection outline and every
    // handle is drawn in.
    let property = "--document-light-selection-handle";
    let rewritten = tokens_source::rewrite(&before, property, "#c02a5f")
        .expect("a hexadecimal colour is a colour");
    let after = rewritten.text;

    let (start, end) = tokens_source::value_span(&before, "document.light.selection-handle")
        .expect("the token is in the source");
    // The span is the **whole JSON value**, quotes included — `"{color.green-deep}"` — and what
    // replaces it is `"#c02a5f"`, nine bytes.
    let written = "\"#c02a5f\"";
    assert_eq!(
        &before[..start],
        &after[..start],
        "everything before the token's value must be byte-identical"
    );
    assert_eq!(
        &before[end..],
        &after[start + written.len()..],
        "everything after the token's value must be byte-identical"
    );
    assert_eq!(&after[start..start + written.len()], written);
    // **The alias warning is live, and this token is the one that proves it.** Its committed value
    // is `{color.green-deep}` — a statement that the selection handle follows the accent, not a
    // colour — so a write-back here changes the token *system* and the harness has to say so.
    assert_eq!(rewritten.previous, "{color.green-deep}");
    assert!(
        rewritten.previous_was_alias,
        "`document.light.selection-handle` is committed as an alias, and a write-back that did not \
         notice would flatten it silently"
    );
    assert_eq!(
        before.len() - (end - start) + written.len(),
        after.len(),
        "the file changed by more than the length of the value"
    );

    // And the file that came out is a file the platform can still read.
    let mut tokens = Tokens::DEFAULTS.clone();
    tokens
        .set_custom_property(property, "#c02a5f")
        .expect("the value the source now carries parses");
    assert_eq!(
        tokens
            .custom_property(property)
            .expect("the token has a value")
            .to_string(),
        "#c02a5f"
    );
}

#[test]
fn a_written_token_is_the_one_that_was_asked_for() {
    // The walk descends key by key, so a segment name that also exists elsewhere cannot be
    // answered by the wrong occurrence. `light` appears under both `theme` and `document`, and
    // `background` under `theme.light` alone — the two are a real collision in this file, not a
    // contrived one.
    let before = source();
    let theme = tokens_source::value_span(&before, "theme.light.background")
        .expect("`theme.light.background` is in the source");
    let document = tokens_source::value_span(&before, "document.light.backdrop")
        .expect("`document.light.backdrop` is in the source");
    assert_ne!(
        theme, document,
        "two different tokens resolved to the same span in the source"
    );
    // What the span actually holds is `{color.paper}` — an **alias**, not a colour. That is not an
    // accident of this token: about a third of the source is written in the W3C alias form, which
    // is the whole reason `Rewritten::previous_was_alias` exists and the reason the harness warns
    // when a write-back flattens one. A test that asserted every span begins with `#` would have
    // been asserting something false about the file it reads.
    let span = tokens_source::value_text(&before, "theme.light.background")
        .expect("`theme.light.background` is in the source");
    assert!(
        span.starts_with('#') || (span.starts_with('{') && span.ends_with('}')),
        "`theme.light.background`'s value is `{span}`, which is neither a colour nor an alias"
    );

    // Every token the generated table names must be findable in the source, or the editor would
    // offer a control that cannot be committed.
    let mut absent = Vec::new();
    for identity in mjx_tokens::TOKENS {
        if tokens_source::value_span(&before, identity.path).is_none() {
            absent.push(identity.path);
        }
    }
    assert!(
        absent.is_empty(),
        "{} token(s) the generated table names are not findable in {}: {}",
        absent.len(),
        tokens_source::SOURCE,
        absent.join(", ")
    );
    println!(
        "\n{} tokens, every one of them reachable in {} and editable in the harness.",
        mjx_tokens::TOKENS.len(),
        tokens_source::SOURCE
    );
}

#[test]
fn a_value_is_written_in_the_shape_its_own_type_calls_for() {
    // **The ten tokens the first version of this editor could not write, and the reason it could
    // not.** `$value` is a quoted string for a colour, a dimension, a duration, a font stack and an
    // alias — and a *bare number* for `font-weight` and `leading`, and an *array* for `ease`. An
    // editor that only knew about strings would offer ninety-two controls and commit eighty-two,
    // silently, which is precisely the shape of hole this phase keeps finding. The suite found it
    // rather than a reader.
    let before = source();

    let weight = tokens_source::rewrite(&before, "--font-weight-medium", "600")
        .expect("600 is a font weight");
    let (start, end) = tokens_source::value_span(&before, "font-weight.medium")
        .expect("`font-weight.medium` is in the source");
    assert_eq!(&before[start..end], "500", "it is a bare number in the source");
    assert_eq!(
        &weight.text[start..start + 3],
        "600",
        "a font weight must be written as a number, not as `\"600\"` — the source is `$type`-annotated"
    );
    assert!(!weight.previous_was_alias);

    let leading = tokens_source::rewrite(&before, "--leading-tight", "1.4").expect("1.4 is a number");
    let (start, end) =
        tokens_source::value_span(&before, "leading.tight").expect("`leading.tight` is in the source");
    assert_eq!(&before[start..end], "1.25");
    assert_eq!(&leading.text[start..start + 3], "1.4");

    let ease = tokens_source::rewrite(&before, "--ease-ink", "cubic-bezier(0.4, 0, 0.2, 1)")
        .expect("a cubic bezier");
    let (start, end) =
        tokens_source::value_span(&before, "ease.ink").expect("`ease.ink` is in the source");
    assert_eq!(&before[start..end], "[0.45, 0, 0.2, 1]");
    assert_eq!(
        &ease.text[start..start + "[0.4, 0, 0.2, 1]".len()],
        "[0.4, 0, 0.2, 1]",
        "an easing curve must be written as the array the source uses, not as its CSS spelling"
    );

    // And whatever shape it took, the file that came out is still JSON the platform reads: each of
    // the three is written back into a source the walk can find again and parse.
    for (rewritten, path, expected) in [
        (&weight, "font-weight.medium", "600"),
        (&leading, "leading.tight", "1.4"),
        (&ease, "ease.ink", "[0.4, 0, 0.2, 1]"),
    ] {
        assert_eq!(
            tokens_source::value_text(&rewritten.text, path).as_deref(),
            Some(expected),
            "`{path}` did not come back out of the file it was written into"
        );
    }
}

#[test]
fn a_bad_write_is_refused_before_a_byte_reaches_the_file() {
    let before = source();

    let unknown = tokens_source::rewrite(&before, "--not-a-token", "#000000");
    assert!(
        matches!(unknown, Err(WriteBackError::UnknownToken { .. })),
        "a name that is not a token must be refused, not written: {unknown:?}"
    );

    // And a literal is reported as a literal, so the warning is not simply always on.
    let literal = tokens_source::rewrite(&before, "--color-ink", "#101010")
        .expect("a hexadecimal colour is a colour");
    assert!(
        !literal.previous_was_alias,
        "`color.ink` is committed as `{}`, which is not an alias",
        literal.previous
    );

    let malformed = tokens_source::rewrite(&before, "--color-ink", "not a colour");
    assert!(
        matches!(malformed, Err(WriteBackError::Malformed(_))),
        "a value that is not a colour must be refused: {malformed:?}"
    );

    // The refusal is what keeps the workspace buildable: a value the resolver rejects is a value
    // that breaks `cargo run -p xtask -- tokens` for whoever runs it next.
    let mut tokens = Tokens::DEFAULTS.clone();
    assert!(tokens
        .set_custom_property("--color-ink", "not a colour")
        .is_err());
}

#[test]
fn the_editor_offers_every_token_and_says_what_kind_each_is() {
    let tokens = Tokens::DEFAULTS.clone();
    let editable = tokens_source::editable(&tokens);
    assert_eq!(
        editable.len(),
        mjx_tokens::TOKENS.len(),
        "the editor offers {} of the platform's {} tokens",
        editable.len(),
        mjx_tokens::TOKENS.len()
    );
    let colours = editable
        .iter()
        .filter(|token| token.kind == "color")
        .count();
    assert!(
        colours > 40,
        "only {colours} tokens are colours, which is fewer than the palette alone has"
    );
    // Every kind the panel can show must actually occur, or the branch that shows it is unreachable.
    for kind in [
        "color",
        "dimension",
        "duration",
        "number",
        "font-weight",
        "font-stack",
    ] {
        assert!(
            editable.iter().any(|token| token.kind == kind),
            "no token is a `{kind}`, so the editor's control for one is unreachable"
        );
    }
}

#[test]
fn a_token_the_canvas_reads_moves_every_scene_that_reads_it() {
    // **The half of the round trip that makes the editor worth having.** A write-back that changed
    // a file and not the picture would be a text editor with extra steps, so this changes the value
    // in memory — exactly as `POST /api/tokens` does — and requires the renders to move.
    use mjx_canvas_harness::inventory::INVENTORY;
    use mjx_canvas_harness::render::{self, Overlays, Scene};
    use mjx_canvas_harness::state::State;
    use mjx_render_oracle::digest::sha256_hex;

    let before = Tokens::DEFAULTS.clone();
    let mut after = Tokens::DEFAULTS.clone();
    after
        .set_custom_property("--document-light-selection-handle", "#c02a5f")
        .expect("a colour");

    let mut moved = 0_usize;
    for entry in &INVENTORY {
        let one = Scene::build(entry, &before, State::CANONICAL);
        let two = Scene::build(entry, &after, State::CANONICAL);
        // The pixels rather than the encoded PNG, for the reason
        // `tests/the_axes_are_not_identities.rs` gives: the encoder is deterministic, so the two
        // answers agree, and one of them is a hand-written deflate over an LZ77 search.
        let first = render::render(&one, Overlays::NONE).expect("renders");
        let second = render::render(&two, Overlays::NONE).expect("renders");
        if sha256_hex(&first.image.rgba) != sha256_hex(&second.image.rgba) {
            moved += 1;
        }
    }
    println!(
        "\nchanging `--document-light-selection-handle` moves {moved} of the {} scenes.",
        INVENTORY.len()
    );
    assert!(
        moved >= 20,
        "changing the selection-handle colour moved only {moved} scenes. It is the colour every \
         selection outline and every handle is drawn in, so a small number here means scenes are \
         carrying literal colours rather than reading the tokens."
    );
}
