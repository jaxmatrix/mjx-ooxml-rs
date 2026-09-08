//! **The hit-test visualiser draws the spatial index's own answer, and nothing else.**
//!
//! MJXOFF-166: *"the hit-test visualiser is proved against the spatial index, not against a second
//! region computation."* That is a rule about where a number comes from, and the only way to check
//! it is to ask the index the same question the overlay asked and require the same answer.
//!
//! [`crate::canvas::Grab`] is shaped so the rule is hard to break: it records a **fragment**, not a
//! rectangle. There is nowhere for a second computation to live. What this suite adds is the two
//! things the shape cannot say by itself:
//!
//! * the index really does answer with that fragment at the region's own centre, so the overlay is
//!   over the thing it claims to be over rather than merely near it; and
//! * the padding is **live** — moving the input axis to touch enlarges every region — because a
//!   grab region that is the same size for a thumb and a mouse is the defect the whole overlay
//!   exists to make visible.
//!
//! [`crate::canvas::Grab`]: mjx_canvas_harness::canvas::Grab

use mjx_canvas_harness::inventory::INVENTORY;
use mjx_canvas_harness::render::{self, Overlays, Scene};
use mjx_canvas_harness::state::{Input, State};
use mjx_layout::SpatialIndex;
use mjx_tokens::Tokens;

#[test]
fn every_grab_region_is_where_the_index_says_the_fragment_is() {
    let tokens = Tokens::DEFAULTS.clone();
    let mut regions = 0_usize;
    let mut entries_with_regions = 0_usize;
    for entry in &INVENTORY {
        let scene = Scene::build(entry, &tokens, State::CANONICAL);
        let grabs = scene.canvas.grabs();
        if grabs.is_empty() {
            continue;
        }
        entries_with_regions += 1;
        let (tree, ids) = scene.canvas.tree();
        let index = SpatialIndex::build(&tree);
        for grab in grabs {
            let fragment = ids[grab.node.index()];
            let bounds = index.bounds_of(fragment).unwrap_or_else(|| {
                panic!(
                    "entry {}: the index has no bounds for `{}`, so the overlay would draw \
                     nothing where a grab region is",
                    entry.number, grab.label
                )
            });
            assert!(
                !bounds.is_empty(),
                "entry {}: `{}` has empty bounds, so its grab region is a region a pointer cannot \
                 be inside",
                entry.number,
                grab.label
            );
            let centre = render::grab_centre(&index, fragment).expect("bounds imply a centre");
            let hits = index.fragments_at(centre);
            assert!(
                hits.contains(&fragment),
                "entry {}: the index does not report `{}` at the centre of its own bounds. The \
                 overlay is drawn from `bounds_of`, so a region the index would not answer for is \
                 a region that is over the wrong thing.",
                entry.number,
                grab.label
            );
            assert!(
                index.fragments_intersecting(bounds).contains(&fragment),
                "entry {}: `{}` does not intersect its own bounds",
                entry.number,
                grab.label
            );
            assert!(
                grab.padding > 0.0,
                "entry {}: `{}` has no padding, so its grab region is exactly its drawn size and \
                 the overlay says nothing a look at the element would not",
                entry.number,
                grab.label
            );
            regions += 1;
        }
    }
    println!(
        "\n{regions} grab regions across {entries_with_regions} of {} entries, every one of them \
         the spatial index's own answer.",
        INVENTORY.len()
    );
    // Twenty entries and seventy-four regions today. The floor is fifteen: low enough that a
    // deliberate redesign of one family does not trip it, high enough that a harness which stopped
    // recording grab regions would be caught rather than reported as *"every grab region checks
    // out"* over an empty set — which is what this assertion is really for.
    assert!(
        entries_with_regions >= 15,
        "only {entries_with_regions} entries record a grab region, so this gate measures almost \
         nothing"
    );
    assert!(
        regions >= 50,
        "only {regions} grab regions in the whole inventory"
    );
}

#[test]
fn a_finger_gets_a_bigger_region_than_a_pointer() {
    // **The identity-value check, applied to the padding.** A grab region computed from a constant
    // would pass every assertion above and would be the same size for a thumb as for a mouse — which
    // is the one thing the hit-test overlay exists to let a person judge.
    let tokens = Tokens::DEFAULTS.clone();
    let mut compared = 0_usize;
    for entry in &INVENTORY {
        let pointer = Scene::build(entry, &tokens, State::CANONICAL);
        let touch = Scene::build(
            entry,
            &tokens,
            State {
                input: Input::Touch,
                ..State::CANONICAL
            },
        );
        if pointer.canvas.grabs().is_empty() {
            continue;
        }
        assert_eq!(
            pointer.canvas.grabs().len(),
            touch.canvas.grabs().len(),
            "entry {}: the two input devices produce different numbers of grab regions. The \
             element is the same element; only its sizes change.",
            entry.number
        );
        for (with_pointer, with_finger) in pointer
            .canvas
            .grabs()
            .iter()
            .zip(touch.canvas.grabs().iter())
        {
            assert!(
                with_finger.padding > with_pointer.padding,
                "entry {}: `{}` reaches {} points past its own bounds for a finger and {} for a \
                 pointer",
                entry.number,
                with_pointer.label,
                with_finger.padding,
                with_pointer.padding
            );
            compared += 1;
        }
    }
    println!("\n{compared} grab regions grow when the input device becomes a finger.");
    assert!(
        compared > 0,
        "no grab region was compared across the input axis"
    );
}

#[test]
fn the_overlay_is_drawn_and_is_not_part_of_the_element() {
    // Two halves of one property. The overlay must *add* something — an overlay that drew nothing
    // would pass a visual review by being invisible — and it must add nothing when it is off, so a
    // plate is a plate of the element.
    let tokens = Tokens::DEFAULTS.clone();
    let entry = INVENTORY
        .iter()
        .find(|entry| {
            !Scene::build(entry, &tokens, State::CANONICAL)
                .canvas
                .grabs()
                .is_empty()
        })
        .expect("at least one entry records a grab region");
    let scene = Scene::build(entry, &tokens, State::CANONICAL);

    let plain = render::render(&scene, Overlays::NONE).expect("the element renders");
    let with_regions = render::render(
        &scene,
        Overlays {
            hit_test: true,
            ruler: false,
        },
    )
    .expect("the overlay renders");
    let with_ruler = render::render(
        &scene,
        Overlays {
            hit_test: false,
            ruler: true,
        },
    )
    .expect("the ruler renders");

    assert!(
        with_regions.list.commands().count() > plain.list.commands().count(),
        "the hit-test overlay added no commands to entry {}",
        entry.number
    );
    assert!(
        with_ruler.list.commands().count() > plain.list.commands().count(),
        "the ruler added no commands to entry {}",
        entry.number
    );
    assert_ne!(
        plain.image, with_regions.image,
        "the hit-test overlay moved no pixel on entry {}",
        entry.number
    );
    assert_ne!(
        plain.image, with_ruler.image,
        "the ruler moved no pixel on entry {}",
        entry.number
    );

    // And the plate path takes neither: `crate::plates` renders at `Overlays::NONE`, so this is the
    // image a baseline holds.
    let again = render::render(&scene, Overlays::NONE).expect("the element renders again");
    assert_eq!(
        plain.image, again.image,
        "two renders of the same scene at the same state differ, so no baseline taken from one \
         could be checked against the other"
    );
}
