//! The facade's handle types, derived from its module layout (MJXOFF-252).
//!
//! # Why this is a module rather than twelve lines in a test
//!
//! `xtask/tests/derived_rosters.rs` closes the class MJXOFF-224 named — a literal list, in test
//! code, of something this repository can enumerate for itself — by sweeping every tracked `.rs`
//! file for a `[…]` whose elements' first **string literals** all belong to a derived population.
//! MJXOFF-252 is the residue that sweep states about itself: *a roster whose elements are not string
//! literals is invisible to it*, and the known instance was
//!
//! ```text
//! const SURFACES: &[&Surface] = &[&DECK, &DOCUMENT, &WORKBOOK];
//! ```
//!
//! in `xtask/tests/facade_curation.rs` — the whole of the facade handle population, three of three,
//! and correct. It is also exactly the shape that was correct in `child_order.rs` the day it was
//! written. A fourth facade handle would join the derived population, join the five string-spelled
//! rosters MJXOFF-225 registered, and **not** join that one.
//!
//! The ticket's own preference was to derive it in place rather than to teach the scanner to read
//! `&DECK` as naming `Deck`, which would be a heuristic rather than a fact. Deriving it needs the
//! derivation to be reachable from two test binaries, and this repository's standing answer to a
//! second consumer is one implementation both can reach — the same reason
//! [`crate::binding_surface`] exists, and the same reason `mjx-allocation-counter` is a crate. Two
//! walks over `crates/mjx-ooxml/src` would disagree with no way to say which was wrong.

use std::collections::BTreeSet;
use std::path::Path;

/// The facade's source directory, relative to the repository root.
pub const FACADE_SOURCE: &str = "crates/mjx-ooxml/src";

/// The facade's handle types, as its own module layout declares them.
///
/// A handle is a module that is big enough to have a directory: `deck.rs` beside `deck/`,
/// `document.rs` beside `document/`, `workbook.rs` beside `workbook/`. The names come back in the
/// module's own spelling (`deck`), not the type's (`Deck`), because that is what is on disk;
/// callers compare case-insensitively.
///
/// # Panics
/// If `crates/mjx-ooxml/src` cannot be read. That is a broken checkout rather than a finding, and
/// silently answering "no handles" would make every comparison against this vacuous.
#[must_use]
pub fn handle_types(root: &Path) -> BTreeSet<String> {
    let facade = root.join(FACADE_SOURCE);
    let mut handles = BTreeSet::new();
    let entries =
        std::fs::read_dir(&facade).unwrap_or_else(|e| panic!("reading {FACADE_SOURCE}: {e}"));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if path.extension().is_some_and(|value| value == "rs") && facade.join(stem).is_dir() {
            handles.insert(stem.to_owned());
        }
    }
    handles
}
