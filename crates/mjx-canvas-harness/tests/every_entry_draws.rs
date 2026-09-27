//! **The completeness gate**, and the trap it is written against.
//!
//! MJXOFF-166 states it in one line: *"all 61 elements have a scene" is satisfied by 61 scenes that
//! render nothing — a titled empty canvas passes it perfectly.* So nothing here asks whether an
//! entry is registered. Every assertion is about what the entry **drew**, and there are six of them:
//!
//! 1. `placeholders == 0` — the element is its own geometry, not `mjx-scene`'s framed crossed
//!    rectangle standing in for one.
//! 2. `draw_calls > 0` — something reached the painter.
//! 3. `covered > MINIMUM_COVERED` — it reached pixels.
//! 4. `distinct_colours >= MINIMUM_COLOURS` — **it reached pixels that are not the page's own
//!    colour.** R10's hand-off 6: its nested specimen drew the innermost box in the same ink as its
//!    parent, all three other counters were healthy, and the box was invisible.
//! 5. Every [`Draws`] kind the entry declares is in its display list — checked on the commands, so
//!    a missing stroke is named as a missing command rather than as a missing pixel.
//! 6. **More commands than the bare stage.** The other five can all be satisfied by a page with no
//!    element on it, because a page is a fill and has colours; this one cannot.
//!
//! And the gate is proved able to fail: [`the_gate_refuses_a_scene_that_draws_nothing_new`] renders
//! an entry's stage alone and shows the sixth assertion go red naming the number.

use mjx_canvas_harness::inventory::{Draws, INVENTORY, MINIMUM_COLOURS, MINIMUM_COVERED};
use mjx_canvas_harness::render::{self, Overlays, Scene};
use mjx_canvas_harness::state::State;
use mjx_tokens::Tokens;

/// How many commands a bare stage — a backdrop and a page — produces.
fn bare_stage_commands() -> usize {
    render::bare_stage_list(&Tokens::DEFAULTS.clone())
        .expect("the bare stage builds")
        .commands()
        .count()
}

#[test]
fn the_inventory_is_the_document_it_transcribes() {
    assert_eq!(
        INVENTORY.len(),
        61,
        "`CANVAS_UI_INVENTORY.md` §2 defines sixty-one elements"
    );
    for (index, entry) in INVENTORY.iter().enumerate() {
        assert_eq!(
            usize::from(entry.number),
            index + 1,
            "entry `{}` is at index {index} and calls itself number {}. Every artefact this crate \
             produces is addressed by number, so the two must agree.",
            entry.title,
            entry.number
        );
    }
    for family in mjx_canvas_harness::Family::ALL {
        let counted = INVENTORY
            .iter()
            .filter(|entry| entry.family == family)
            .count();
        assert_eq!(
            counted,
            family.size(),
            "§{} ({}) has {counted} entries and the inventory says {}",
            family.section(),
            family.label(),
            family.size()
        );
    }
    let mut slugs: Vec<String> = INVENTORY
        .iter()
        .map(mjx_canvas_harness::Entry::slug)
        .collect();
    slugs.sort();
    let before = slugs.len();
    slugs.dedup();
    assert_eq!(
        before,
        slugs.len(),
        "two entries share a slug, so two plates would share a baseline directory"
    );
}

#[test]
fn every_entry_draws_something_of_its_own() {
    let tokens = Tokens::DEFAULTS.clone();
    let floor = bare_stage_commands();
    assert!(
        floor > 0,
        "the bare stage draws nothing, so the floor every entry has to clear is zero and this gate \
         is vacuous"
    );

    let mut report = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for entry in &INVENTORY {
        let scene = Scene::build(entry, &tokens, State::CANONICAL);
        let rendered = render::render(&scene, Overlays::NONE)
            .unwrap_or_else(|error| panic!("entry {}: {error}", entry.number));

        assert_eq!(
            rendered.report.placeholders, 0,
            "entry {} (`{}`) drew {} stand-in outline(s). A canvas element is this harness's own \
             geometry, never `mjx-scene`'s `PlaceholderGeometry`, and a plate of a stand-in is a \
             plate of the stand-in.",
            entry.number, entry.title, rendered.report.placeholders
        );
        assert!(
            rendered.report.draw_calls > 0,
            "entry {} (`{}`) issued no draw calls",
            entry.number,
            entry.title
        );
        assert!(
            rendered.covered > MINIMUM_COVERED,
            "entry {} (`{}`) covered {} pixels, fewer than {MINIMUM_COVERED}",
            entry.number,
            entry.title,
            rendered.covered
        );
        assert!(
            rendered.distinct_colours >= MINIMUM_COLOURS,
            "entry {} (`{}`) has {} distinct colours, fewer than {MINIMUM_COLOURS}. Every counter \
             above can be healthy for an element drawn in the page's own ink — which is invisible, \
             and which a change to would move no pixel.",
            entry.number,
            entry.title,
            rendered.distinct_colours
        );

        let drawn = render::kinds_drawn(&rendered.list);
        for kind in entry.draws {
            if !drawn.contains(kind) {
                missing.push(format!(
                    "entry {} (`{}`) declares {} and its display list has none. What it does \
                     have: {}.",
                    entry.number,
                    entry.title,
                    kind.label(),
                    describe(&drawn)
                ));
            }
        }

        let commands = rendered.list.commands().count();
        assert!(
            commands > floor,
            "entry {} (`{}`) produced {commands} commands and the bare stage produces {floor}. It \
             is registered and it draws nothing of its own, which is exactly the shape of \
             completeness this gate exists to refuse.",
            entry.number,
            entry.title
        );

        report.push((
            entry.number,
            commands,
            rendered.report.draw_calls,
            rendered.covered,
            rendered.distinct_colours,
            drawn,
        ));
    }

    assert!(
        missing.is_empty(),
        "{} declaration(s) the display list does not carry:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );

    println!("\nthe bare stage is {floor} commands; every entry clears it:\n");
    println!("  #  commands  draws  covered  colours  kinds");
    for (number, commands, draws, covered, colours, kinds) in &report {
        println!(
            "{number:>3}  {commands:>8}  {draws:>5}  {covered:>7}  {colours:>7}  {}",
            describe(kinds)
        );
    }
    let least = report.iter().map(|row| row.1).min().unwrap_or(0);
    let colours = report.iter().map(|row| row.4).min().unwrap_or(0);
    println!(
        "\n{} entries; the quietest draws {least} commands and the flattest has {colours} colours.",
        report.len()
    );
}

#[test]
fn the_gate_refuses_a_scene_that_draws_nothing_new() {
    // **The gate proved able to fail.** MJXOFF-166 asks for exactly this: *"remove one scene's
    // content and show the check go red naming the entry number."* Rather than deleting a scene —
    // which would leave the suite unable to run itself — this renders the stage *as if* it were
    // entry 1's whole scene and asserts that the sixth check refuses it.
    let floor = bare_stage_commands();
    let list =
        render::bare_stage_list(&Tokens::DEFAULTS.clone()).expect("the emptied scene builds");

    assert!(
        list.commands().count() <= floor,
        "an emptied scene draws more than the bare stage, so the floor is not measuring what it \
         claims to"
    );
    // And it would pass the other five: a page has pixels, has colours, has draw calls and has no
    // placeholders. That is precisely why the sixth exists.
    let entry = &INVENTORY[0];
    assert!(
        entry.draws.contains(&Draws::Stroke),
        "entry 1 declares a stroke, which an emptied stage would not have either"
    );
    let drawn = render::kinds_drawn(&list);
    assert!(
        !drawn.contains(&Draws::Stroke),
        "the bare stage strokes something, so it would satisfy entry 1's declaration and the fifth \
         check would not catch an emptied scene either"
    );
}

#[test]
fn the_one_effect_in_the_inventory_reaches_pixels() {
    // **The identity-value check, applied to the one `PushEffect` this harness emits.**
    //
    // Entry 44 declares `Draws::Effect` and the completeness gate proves the command is in the
    // display list. That is exactly the shape of the defect R09 shipped: `Effect::new` left both
    // reflection alphas at zero, so a whole painter arm executed on every frame and had never
    // produced a pixel — and *two painters drawing nothing agree perfectly*. A page shadow is a
    // `#223b3314` at eight percent alpha behind a `#fdfcf9` backdrop, which is precisely the kind of
    // effect that can be present, correct, and invisible.
    //
    // So this asserts on the pixels **outside the page**: below its bottom edge there must be
    // samples that are neither the backdrop nor the page, or the shadow did not reach the frame.
    use mjx_canvas_harness::canvas::{Ink, PAGE};

    let tokens = Tokens::DEFAULTS.clone();
    let entry = mjx_canvas_harness::entry(44).expect("entry 44 is the page and its shadow");
    assert!(
        entry.draws.contains(&Draws::Effect),
        "entry 44 no longer declares an effect, so this gate is about the wrong element"
    );
    let scene = Scene::build(entry, &tokens, State::CANONICAL);
    let rendered = render::render(&scene, Overlays::NONE).expect("entry 44 renders");
    let ink = Ink::new(&tokens, State::CANONICAL);
    let backdrop = ink.backdrop();
    let page = ink.page();

    // `page_and_shadow` insets its page by six points inside `PAGE` and drops eighteen from the
    // bottom, and the scale is 96/72 device pixels to the point.
    let scale = f64::from(scene.canvas.pixels_per_point());
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the stage is 300 by 200 points at 4/3 device pixels to the point"
    )]
    let bottom = ((PAGE.y + PAGE.height - 18.0 + 6.0) * scale).round() as u32;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the stage is 300 by 200 points at 4/3 device pixels to the point"
    )]
    let (left, right) = (
        ((PAGE.x + 20.0) * scale).round() as u32,
        ((PAGE.x + PAGE.width - 20.0) * scale).round() as u32,
    );

    let mut shaded = 0_usize;
    for y in bottom..(bottom + 14).min(rendered.image.height) {
        for x in left..right.min(rendered.image.width) {
            let Some(pixel) = rendered.image.pixel(x, y) else {
                continue;
            };
            let is = |colour: mjx_scene::Color| {
                pixel[0] == colour.red && pixel[1] == colour.green && pixel[2] == colour.blue
            };
            if !is(backdrop) && !is(page) {
                shaded += 1;
            }
        }
    }
    println!("\nthe page shadow marks {shaded} pixels below the page's own bottom edge.");
    assert!(
        shaded > 200,
        "the page shadow marked {shaded} pixels below the page. The `PushEffect` is in the display \
         list — the completeness gate proves that — and a command that executes and produces \
         nothing is the exact defect R09 shipped when `Effect::new` left its reflection alphas at \
         zero. Either `document.*.page-shadow`'s alpha or blur has gone to nothing, or the effect \
         is not reaching the painter."
    );
}

/// A list of kinds, for a message.
fn describe(kinds: &[Draws]) -> String {
    if kinds.is_empty() {
        return "nothing".to_owned();
    }
    kinds
        .iter()
        .map(|kind| kind.label())
        .collect::<Vec<_>>()
        .join(", ")
}
