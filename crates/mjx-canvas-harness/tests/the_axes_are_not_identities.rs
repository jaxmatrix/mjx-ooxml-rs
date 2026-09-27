//! **The state matrix, measured.** *How many distinct values did the gate actually see?*
//!
//! # The defect class this suite exists for
//!
//! A parameter reached constantly but only ever at its no-op value is invisible to any reachability
//! check: the code runs, the coverage is green, and the parameter has never once done anything. R09
//! shipped exactly that — `Effect::new` left both reflection alphas at zero, so a whole painter arm
//! had never produced a pixel, and *two painters drawing nothing agree perfectly*.
//!
//! A state panel with four toggles is four such parameters, sixty-one times over. So:
//!
//! * Each [`Entry`] **declares** which axes move its pixels, and this suite measures the
//!   declaration against the renders **in both directions**. An entry that claims to respond to
//!   hover and does not is a toggle that does nothing; one that responds without saying so is a
//!   state nobody was told to audit. Both fail, and both name the number.
//! * The aggregate counts are **printed**, so *"61 elements exercised"* is a number somebody can
//!   check rather than a sentence. If the interaction axis moved three scenes, the report says
//!   three.
//! * The floors are stated as constants with reasons. They are not a substitute for the
//!   declarations — they are what catches a declaration table that was *quietly* relaxed one entry
//!   at a time until it said nothing.
//!
//! # ⚠ Why density is measured differently from the other three
//!
//! Rendering at 2× changes the image's dimensions, so *"the render differs"* is true of every scene
//! by construction and would be a vacuous assertion. What the density axis is actually **for** is
//! whether a hairline survives, and that question has a sharp mechanical form: a
//! [`mjx_scene::StrokeStyle`]'s width is in **device pixels**, passed through `build_scene`
//! untouched, so a scene that wrote `StrokeStyle::solid(1.0, …)` would draw the same one-pixel line
//! at every density — crisp at 1×, invisible at 3×, and identical in all three.
//! [`every_stroke_scales_with_the_density`] reads the widths out of the display list at each
//! density and requires them to be in proportion.

use std::collections::BTreeSet;

use mjx_canvas_harness::inventory::{Entry, INVENTORY};
use mjx_canvas_harness::render::{self, Overlays, Scene};
use mjx_canvas_harness::state::{Axis, Density, State};
use mjx_render_oracle::digest::sha256_hex;
use mjx_scene::{ResourceIndex, SectionKind};
use mjx_tokens::Tokens;

/// How many entries must respond to the interaction axis.
///
/// Forty-five, against fifty-one that declare it today. The margin is deliberate and so is its
/// direction: the per-entry check above already holds every individual declaration, so a floor equal
/// to the count would say nothing new. What this catches is the *table* being relaxed — one entry at
/// a time, each change locally defensible — until the state panel's first toggle does nothing
/// anywhere and every individual assertion still passes.
const INTERACTION_FLOOR: usize = 45;

/// How many entries must respond to the input axis.
///
/// Eleven. `CANVAS_UI_INVENTORY.md` §3 says *"eleven inventory entries are specifically about touch
/// behaviour"*, and this is that sentence as a gate. A harness in which the touch toggle moved ten
/// elements would be one where an element the document says is about touch is not. Nineteen respond
/// today, because sizing an affordance for a finger turned out to be right for more elements than
/// the document called out — which is a reason to keep the floor at the document's number rather
/// than to raise it to ours.
const TOUCH_FLOOR: usize = 11;

/// The SHA-256 of one entry's **pixels** at one state.
///
/// Of the straight-alpha samples rather than of the encoded PNG, and the difference is four minutes
/// of wall clock across this file: `mjx_render_oracle::png::encode` is a hand-written fixed-Huffman
/// deflate over an LZ77 search, which is exactly what a byte-reproducible plate wants and is far
/// more work than a comparison needs. The two answers agree — the encoder is deterministic, which
/// that crate asserts — so nothing is given up by hashing its input instead of its output.
fn digest(entry: &'static Entry, tokens: &Tokens, state: State) -> String {
    let scene = Scene::build(entry, tokens, state);
    let rendered = render::render(&scene, Overlays::NONE)
        .unwrap_or_else(|error| panic!("entry {}: {error}", entry.number));
    sha256_hex(&rendered.image.rgba)
}

#[test]
fn every_declared_axis_moves_a_pixel_and_every_moved_pixel_is_declared() {
    let tokens = Tokens::DEFAULTS.clone();
    let mut wrong: Vec<String> = Vec::new();
    let mut responded: Vec<(Axis, usize)> = Axis::ALL.map(|axis| (axis, 0)).to_vec();

    for entry in &INVENTORY {
        let resting = digest(entry, &tokens, State::CANONICAL);
        for axis in Axis::ALL {
            // **Every value of the axis, not the first alternative.** An element whose hover state
            // is identical and whose focused state is not still responds to the interaction toggle,
            // and a measurement that only moved one step would call it inert — which is how a gate
            // ends up disagreeing with a declaration that is right.
            //
            // Density is the exception and is `true` by construction: rendering at 2× changes the
            // image's dimensions, so *"the render differs"* is true of every scene and would be a
            // vacuous measurement. What the density axis is actually for is measured by
            // `every_stroke_scales_with_the_density`, below.
            let responds = axis == Axis::Density
                || State::CANONICAL
                    .along(axis)
                    .into_iter()
                    .any(|state| digest(entry, &tokens, state) != resting);
            let declared = entry.responds.contains(&axis);
            if responds {
                if let Some(slot) = responded.iter_mut().find(|(name, _)| *name == axis) {
                    slot.1 += 1;
                }
            }
            match (declared, responds) {
                (true, false) => wrong.push(format!(
                    "entry {} (`{}`) declares that {axis} moves its pixels and it does not. The \
                     toggle is in the panel and does nothing.",
                    entry.number, entry.title
                )),
                (false, true) => wrong.push(format!(
                    "entry {} (`{}`) responds to {axis} and does not say so. A state nobody was \
                     told to audit is a state nobody audits.",
                    entry.number, entry.title
                )),
                _ => {}
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "{} entry/axis disagreement(s) between what is declared and what was measured:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );

    println!(
        "\nhow many of the {} entries each axis moves:",
        INVENTORY.len()
    );
    for (axis, count) in &responded {
        println!("  {axis:<12} {count:>3}");
    }

    let interaction = responded
        .iter()
        .find(|(axis, _)| *axis == Axis::Interaction)
        .map_or(0, |(_, count)| *count);
    let input = responded
        .iter()
        .find(|(axis, _)| *axis == Axis::Input)
        .map_or(0, |(_, count)| *count);
    let scheme = responded
        .iter()
        .find(|(axis, _)| *axis == Axis::Scheme)
        .map_or(0, |(_, count)| *count);

    assert_eq!(
        scheme,
        INVENTORY.len(),
        "{scheme} of {} entries change with the colour scheme. Every scene sits on a themed \
         backdrop and reads its colours from `mjx-tokens`, so an entry that does not change is one \
         that has a colour written into it.",
        INVENTORY.len()
    );
    assert!(
        interaction >= INTERACTION_FLOOR,
        "only {interaction} entries respond to the interaction toggle, and the floor is \
         {INTERACTION_FLOOR}"
    );
    assert!(
        input >= TOUCH_FLOOR,
        "only {input} entries respond to the input toggle, and `CANVAS_UI_INVENTORY.md` §3 says \
         eleven of them are specifically about touch behaviour"
    );
}

#[test]
fn the_sixty_one_renders_are_sixty_one_different_pictures() {
    // **The direct answer to *"something must prove it was 61 and not 3"*.** A harness whose scenes
    // all drew the same page would satisfy every counter in `every_entry_draws.rs` — each scene
    // would have its own commands, its own colours and its own coverage — and would be one
    // element audited sixty-one times.
    let tokens = Tokens::DEFAULTS.clone();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut duplicates = Vec::new();
    for entry in &INVENTORY {
        let digest = digest(entry, &tokens, State::CANONICAL);
        if !seen.insert(digest) {
            duplicates.push(format!("{} (`{}`)", entry.number, entry.title));
        }
    }
    assert!(
        duplicates.is_empty(),
        "{} entries render the same picture as an earlier one: {}",
        duplicates.len(),
        duplicates.join(", ")
    );
    assert_eq!(seen.len(), INVENTORY.len());
    println!(
        "\n{} entries, {} distinct renders at the canonical state.",
        INVENTORY.len(),
        seen.len()
    );
}

#[test]
fn every_stroke_scales_with_the_density() {
    // See this module's own documentation: a stroke width is in **device pixels**, so this is the
    // gate that catches a scene which wrote one instead of converting from points.
    let tokens = Tokens::DEFAULTS.clone();
    let mut stroked = 0_usize;
    let mut wrong = Vec::new();
    for entry in &INVENTORY {
        let widths = |density: Density| -> Vec<f32> {
            let state = State {
                density,
                ..State::CANONICAL
            };
            let scene = Scene::build(entry, &tokens, state);
            // The display list alone. This gate asks only about stroke widths, and rasterising a
            // 1200 by 800 frame to read a number out of the command stream would make the suite
            // several minutes slower for no additional answer.
            let list = render::display_list(&scene, Overlays::NONE)
                .unwrap_or_else(|error| panic!("entry {}: {error}", entry.number));
            let count = list.record_count(SectionKind::Strokes);
            let mut widths: Vec<f32> = (0..count)
                .filter_map(|index| list.stroke(ResourceIndex::new(index)))
                .map(|stroke| stroke.width)
                .collect();
            widths.sort_by(f32::total_cmp);
            widths
        };
        let single = widths(Density::One);
        let double = widths(Density::Two);
        let triple = widths(Density::Three);
        if single.is_empty() {
            continue;
        }
        stroked += 1;
        if single.len() != double.len() || single.len() != triple.len() {
            wrong.push(format!(
                "entry {} (`{}`) has {} stroke(s) at 1×, {} at 2× and {} at 3×. The drawing is not \
                 meant to change with the density, only its resolution.",
                entry.number,
                entry.title,
                single.len(),
                double.len(),
                triple.len()
            ));
            continue;
        }
        for (index, width) in single.iter().enumerate() {
            for (factor, other) in [(2.0_f32, &double), (3.0, &triple)] {
                let expected = width * factor;
                if (other[index] - expected).abs() > 0.01 {
                    wrong.push(format!(
                        "entry {} (`{}`): a stroke {width} device pixels wide at 1× is \
                         {} at {factor}× and should be {expected}. A width stated in device pixels \
                         rather than converted from points is a hairline that is crisp at 1× and \
                         invisible at 3×.",
                        entry.number, entry.title, other[index]
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n  "));
    assert!(
        stroked >= 50,
        "only {stroked} of {} entries stroke anything, so this gate measures almost nothing",
        INVENTORY.len()
    );
    println!(
        "\n{stroked} entries stroke something, and every width is in proportion at 1×, 2× and 3×."
    );
}
