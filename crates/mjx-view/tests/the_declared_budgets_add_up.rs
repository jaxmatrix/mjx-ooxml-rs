//! The seven declared ceilings add up to the two figures `docs/UI_PLATFORM_PLAN.md` §12 states.
//!
//! # Why a table of constants needs a test
//!
//! [`CacheBudget::desktop`] and [`CacheBudget::mobile`] divide §12's budgets across seven stages.
//! Nothing about that division is checkable by the compiler, so the failure mode is quiet: someone
//! raises the glyph atlas because a page looked wrong, and the client's resident-memory budget is
//! now 448 MB and the document still says 400. This file is what makes that a red build instead of
//! a number nobody adds up again.
//!
//! It also asserts the thing the *type* cannot: that four of the seven stages are declared here and
//! held elsewhere, and that [`Stage::is_held_by_the_viewport`] agrees with what
//! [`DocumentView::cache_report`] actually reports. A table that named seven stages and a report
//! that covered three, with nothing joining them, would let the two drift apart silently.

use mjx_layout::LayoutSize;
use mjx_ooxml_core::measure::Emu;
use mjx_view::{CacheBudget, DocumentView, Stage, Viewport};

#[path = "support/mod.rs"]
mod support;

use support::{letter, FlowModel, Paragraphs, PlainScenes};

/// One mebibyte.
const MIB: usize = 1024 * 1024;

/// §12: *Resident memory, 400-page document — < 400 MB desktop, < 200 MB mobile.*
const RESIDENT_DESKTOP: usize = 400 * MIB;
const RESIDENT_MOBILE: usize = 200 * MIB;

/// §12: *GPU texture budget — < 256 MB desktop, < 96 MB mobile.*
const TEXTURE_DESKTOP: usize = 256 * MIB;
const TEXTURE_MOBILE: usize = 96 * MIB;

/// The three stages whose bytes are ordinary heap.
const HEAP_STAGES: [Stage; 3] = [Stage::Checkpoints, Stage::Fragments, Stage::DisplayLists];

/// The four whose bytes end up on a device.
///
/// Tessellations are vertex buffers, the atlas and the effect pool are textures, and an image decode
/// is one upload away from being one. They are budgeted against §12's *texture* figure rather than
/// its resident-memory figure, which is why the two sums below are taken separately.
const DEVICE_STAGES: [Stage; 4] = [
    Stage::Tessellations,
    Stage::GlyphAtlas,
    Stage::ImageDecodes,
    Stage::EffectTextures,
];

fn sum(budget: &CacheBudget, stages: &[Stage]) -> usize {
    stages
        .iter()
        .fold(0usize, |total, stage| total + budget.ceiling(*stage))
}

#[test]
fn the_desktop_and_mobile_tables_match_the_figures_in_the_plan() {
    let desktop = CacheBudget::desktop();
    assert!(
        sum(&desktop, &HEAP_STAGES) <= RESIDENT_DESKTOP,
        "the desktop heap stages total {} bytes against §12's {RESIDENT_DESKTOP}",
        sum(&desktop, &HEAP_STAGES),
    );
    assert!(
        sum(&desktop, &DEVICE_STAGES) <= TEXTURE_DESKTOP,
        "the desktop device stages total {} bytes against §12's {TEXTURE_DESKTOP}",
        sum(&desktop, &DEVICE_STAGES),
    );

    let mobile = CacheBudget::mobile();
    assert!(
        sum(&mobile, &HEAP_STAGES) <= RESIDENT_MOBILE,
        "the mobile heap stages total {} bytes against §12's {RESIDENT_MOBILE}",
        sum(&mobile, &HEAP_STAGES),
    );
    assert!(
        sum(&mobile, &DEVICE_STAGES) <= TEXTURE_MOBILE,
        "the mobile device stages total {} bytes against §12's {TEXTURE_MOBILE}",
        sum(&mobile, &DEVICE_STAGES),
    );

    // Mobile is smaller than desktop everywhere, which is the only relationship between the two
    // tables that is a *rule* rather than a number.
    for stage in Stage::ALL {
        assert!(
            mobile.ceiling(stage) < desktop.ceiling(stage),
            "the mobile ceiling for {} is not below the desktop one",
            stage.label(),
        );
    }

    // Every stage is covered by exactly one of the two sums, so a stage added to `Stage::ALL`
    // without a decision about which budget it spends fails here.
    assert_eq!(HEAP_STAGES.len() + DEVICE_STAGES.len(), Stage::ALL.len());
}

#[test]
fn an_unbounded_budget_saturates_rather_than_wrapping() {
    // The one arithmetic mistake that would turn the eviction-disabled control in
    // `tests/resident_memory.rs` into a passing test: seven `usize::MAX`s added up wrap to a small
    // number, and a small number is a budget that evicts.
    let unbounded = CacheBudget::unbounded();
    assert_eq!(unbounded.total(), usize::MAX);
    assert_eq!(unbounded.viewport_total(), usize::MAX);
    for stage in Stage::ALL {
        assert_eq!(unbounded.ceiling(stage), usize::MAX);
    }
}

#[test]
fn the_stages_a_viewport_says_it_holds_are_the_stages_it_reports() {
    // The join between the table and the report. Four of the seven belong to other crates —
    // `mjx-scene`'s `MeshCache`, and the painter's atlas, image and effect pools — and this is what
    // stops the two lists drifting apart.
    let content = Paragraphs::of(4).with_blocks(4);
    let constraints = letter();
    let view = DocumentView::new(
        FlowModel::flowing(),
        PlainScenes::default(),
        &content,
        constraints,
        Viewport::new(LayoutSize {
            width: constraints.page.width,
            height: constraints.page.height,
        }),
        CacheBudget::desktop(),
    );
    let report = view.cache_report();
    for stage in Stage::ALL {
        assert_eq!(
            report.stage(stage).is_some(),
            stage.is_held_by_the_viewport(),
            "{} is reported by the viewport but not declared as held by it, or the reverse",
            stage.label(),
        );
    }
    assert_eq!(report.stages().len(), 3);
    assert_eq!(
        report.bytes(),
        0,
        "a view that has run no frame is holding something",
    );
    assert!(report.within_ceilings());
    assert!(report.first_overrun().is_none());

    // And the declared ceilings really did reach the caches, rather than being carried and ignored.
    assert_eq!(view.budget().fragments, CacheBudget::desktop().fragments);
    assert_eq!(
        report.stage(Stage::Fragments).expect("the stage").ceiling,
        CacheBudget::desktop().fragments,
    );
}

/// A compile-time reminder that a budget is bytes and never a length.
const _: Option<Emu> = None;
