//! The sixty-one scenes, and the one function that dispatches to them.
//!
//! # Why there is a `match` on a number rather than a function pointer on [`Entry`]
//!
//! A `fn(&mut Canvas)` in the inventory table would be tidier and would make one thing impossible to
//! check: that **every** number has a scene. A table of function pointers is complete by
//! construction — you cannot write a row without one — so the completeness gate would have nothing
//! to say. An exhaustive `match` on `1..=61` with a `panic!` arm, plus
//! `tests/every_entry_draws.rs` walking [`crate::inventory::INVENTORY`], is a gate that can
//! actually fail: add an entry to the inventory and forget the scene, and the suite says which
//! number.
//!
//! # ⚠ No scene contains text, no scene needs a document, and no scene names a format
//!
//! Three properties, and each is load-bearing for a different reason.
//!
//! * **No text**, because a golden image containing glyph coverage is a golden image of a rasteriser
//!   version and an installed face — MJXOFF-165's own decision, inherited. See `crate::canvas`.
//! * **No document**, because `CANVAS_UI_INVENTORY.md` §3 requires it: *"it means the harness works
//!   from the moment `mjx-scene` and `mjx-paint` exist, long before any of the three layout engines
//!   do, so in-canvas UI can be audited and settled while the format renderers are still being
//!   built."* A harness that needed a `.pptx` could not run until R14.
//! * **No format crate named**, and no `mjx-geometry` either. A scene draws its own rounded
//!   rectangle rather than asking a preset table for one, because above a `FragmentTree` nothing has
//!   heard of a `.pptx` — and a harness that reached for DrawingML to draw a selection handle would
//!   be the first thing above the seam to cross it.

pub mod feedback;
pub mod furniture;
pub mod grid;
pub mod manipulation;
pub mod object_selection;
pub mod stage;
pub mod text_editing;

use crate::canvas::Canvas;
use crate::inventory::Entry;

/// Draw `entry`'s scene onto `canvas`.
///
/// # Panics
///
/// On an inventory number with no scene — which is a programming error this crate cannot recover
/// from and `tests/every_entry_draws.rs` catches long before a person runs the harness. The panic
/// names the number and the title, so the fix is the next line of the message.
pub fn draw(entry: &Entry, canvas: &mut Canvas) {
    match entry.number {
        1 => object_selection::selection_outline_single(canvas),
        2 => object_selection::selection_outline_multiple(canvas),
        3 => object_selection::resize_handles(canvas),
        4 => object_selection::rotation_handle(canvas),
        5 => object_selection::rotation_snap(canvas),
        6 => object_selection::adjustment_handles(canvas),
        7 => object_selection::connection_sites(canvas),
        8 => object_selection::connector_endpoints(canvas),
        9 => object_selection::group_versus_child(canvas),
        10 => object_selection::locked_object(canvas),
        11 => object_selection::off_canvas_indicator(canvas),

        12 => text_editing::caret_hairline(canvas),
        13 => text_editing::caret_bidi(canvas),
        14 => text_editing::selection_fill_runs(canvas),
        15 => text_editing::selection_fill_across(canvas),
        16 => text_editing::ime_composition(canvas),
        17 => text_editing::squiggles(canvas),
        18 => text_editing::hyperlink_hover(canvas),
        19 => text_editing::text_overflow(canvas),
        20 => text_editing::placeholder_prompt(canvas),
        21 => text_editing::autoscroll_edge(canvas),

        22 => grid::active_cell_border(canvas),
        23 => grid::range_selection(canvas),
        24 => grid::multi_range_selection(canvas),
        25 => grid::fill_handle(canvas),
        26 => grid::header_highlight(canvas),
        27 => grid::pane_dividers(canvas),
        28 => grid::merged_cell_selection(canvas),
        29 => grid::autofilter_dropdown(canvas),
        30 => grid::comment_indicator(canvas),

        31 => manipulation::drag_ghost(canvas),
        32 => manipulation::alignment_guides(canvas),
        33 => manipulation::snap_indicators(canvas),
        34 => manipulation::drag_readout(canvas),
        35 => manipulation::resize_ghost(canvas),
        36 => manipulation::crop_handles(canvas),
        37 => manipulation::table_resize(canvas),
        38 => manipulation::table_insert(canvas),
        39 => manipulation::vertex_editing(canvas),
        40 => manipulation::motion_path_editing(canvas),
        41 => manipulation::marching_ants(canvas),
        42 => manipulation::drop_indicator(canvas),
        43 => manipulation::multi_touch_transform(canvas),

        44 => furniture::page_and_shadow(canvas),
        45 => furniture::page_gap_and_break(canvas),
        46 => furniture::header_footer_regions(canvas),
        47 => furniture::wrap_boundary(canvas),
        48 => furniture::column_boundaries(canvas),
        49 => furniture::section_break_markers(canvas),
        50 => furniture::footnote_separator(canvas),
        51 => furniture::non_printing_marks(canvas),
        52 => furniture::field_shading(canvas),
        53 => furniture::bookmark_brackets(canvas),
        54 => furniture::comment_anchor(canvas),
        55 => furniture::tracked_changes(canvas),
        56 => furniture::guides_and_safe_area(canvas),

        57 => feedback::canvas_focus_ring(canvas),
        58 => feedback::page_placeholder(canvas),
        59 => feedback::missing_resource(canvas),
        60 => feedback::font_substitution_badge(canvas),
        61 => feedback::print_area_hairlines(canvas),

        other => panic!(
            "inventory entry {other} (`{}`) has no scene. Add one to `crate::scenes` and dispatch \
             it here: an entry with no scene is an element nobody can audit, which is exactly what \
             `CANVAS_UI_INVENTORY.md` calls not covered.",
            entry.title
        ),
    }
}

/// The bare stage — a backdrop and a page, and nothing else.
///
/// **The floor every entry has to clear.** `tests/every_entry_draws.rs` renders this, counts its
/// commands, and requires every one of the sixty-one to draw more. A titled empty canvas passes
/// *"all 61 elements have a scene"* perfectly and fails this.
pub fn bare_stage(canvas: &mut Canvas) {
    stage::stage(canvas);
}
