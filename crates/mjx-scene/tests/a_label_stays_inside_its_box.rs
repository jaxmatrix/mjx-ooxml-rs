//! A placeholder's label is set inside the element's own space, at every size a box can be (MJXOFF-299).
//!
//! D8 puts the placeholder in the element's own space, and a label that covers content which did
//! render is worse than no label. So the question this asks is not whether the label reads — that is
//! `mjx-paint`'s, in pixels — but whether one letter of it can ever fall outside the box it was set
//! in. The sweep is over every label a loss carries and every box shape a cell, a frame or a slide
//! produces, because the sizes that go wrong are the small ones nobody renders by hand.

use mjx_scene::{placeholder_lettering, SceneRect};

/// Every label a loss can carry, longest first, each with the short form it falls back to.
const LABELS: [(&str, &str); 9] = [
    ("Embedded object not rendered", "Object"),
    ("Fill picture not available", "Picture"),
    ("Shape outline not resolved", "Outline"),
    ("Text colour approximated", "Text"),
    ("Picture not rendered", "Picture"),
    ("Colour not resolved", "Colour"),
    ("Chart not rendered", "Chart"),
    ("Content not read", "Content"),
    ("Text not shaped", "Text"),
];

/// The widths a real placeholder is asked for: a sliver, an Excel cell, a frame, a slide.
const WIDTHS: [f32; 10] = [2.0, 5.0, 9.0, 16.0, 30.0, 61.0, 96.0, 200.0, 384.0, 672.0];

/// The heights, including the seven and eight pixel boxes where one line of lettering does not fit.
const HEIGHTS: [f32; 10] = [1.0, 3.0, 7.0, 8.0, 9.0, 12.0, 20.0, 48.0, 120.0, 480.0];

#[test]
fn no_label_puts_a_letter_or_its_plate_outside_the_box() {
    let mut set_count = 0_usize;
    for (label, short) in LABELS {
        for width in WIDTHS {
            for height in HEIGHTS {
                // An origin that is not the page's, so an overflow cannot hide behind zero.
                let within = SceneRect::new(7.0, 5.0, 7.0 + width, 5.0 + height);
                let Some(lettering) = placeholder_lettering(label, short, within) else {
                    continue;
                };
                assert!(
                    lettering.text == label || lettering.text == short,
                    "{label:?} in a {width} by {height} box: the letters read {:?}, which is \
                     neither the label nor its short form — a word was cut",
                    lettering.text
                );
                set_count += 1;
                let letters = lettering.ink.bounds();
                assert!(
                    letters.left >= within.left
                        && letters.right <= within.right
                        && letters.top >= within.top
                        && letters.bottom <= within.bottom,
                    "{label:?} in a {width} by {height} box: the letters cover {letters:?}, which \
                     leaves {within:?}"
                );
                assert!(
                    lettering.plate.left >= within.left
                        && lettering.plate.right <= within.right
                        && lettering.plate.top >= within.top
                        && lettering.plate.bottom <= within.bottom,
                    "{label:?} in a {width} by {height} box: the plate covers {:?}, which leaves \
                     {within:?}",
                    lettering.plate
                );
            }
        }
    }
    assert!(
        set_count > 100,
        "only {set_count} of the boxes carried a label at all, so this sweep asserts almost nothing"
    );
}
