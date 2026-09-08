//! The sixty-one plates, and the baseline store they are checked against.
//!
//! # ⚠ Nothing here is new machinery, and that is the point
//!
//! `mjx-render-oracle` already owns every hard part: the PNG encoder and its premultiplication
//! decision, the perceptual metric with its worst-window bound, the manifest schema a loader reads,
//! the baseline store, the approval event that binds a SHA-256, and the regeneration path that
//! **removes** the approval it overwrites. MJXOFF-166 is specified to *reach the plate generator
//! rather than write a second PNG emitter*, and this module is that: it renders sixty-one scenes and
//! hands them to [`mjx_render_oracle::plate::Plate`] and [`mjx_render_oracle::baseline::Baselines`]
//! unchanged.
//!
//! In particular **the manifest is R10's, verbatim.** The hand-off says it plainly: *"your 61
//! entries extend `plates`; do not invent a second schema"* — U01's Storybook loader reads one
//! manifest, not two. So [`plate_set`] builds a [`mjx_render_oracle::plate::PlateSet`] and calls its
//! own `manifest()`; nothing in this file writes JSON.
//!
//! # What a plate is taken at, and what it is not taken with
//!
//! [`crate::state::State::CANONICAL`] — at rest, light, 1×, pointer — with
//! [`crate::render::Overlays::NONE`]. The other fifty-nine points of the matrix are reachable live
//! in the harness and have no plates, because 61 × 60 golden images would be a regression gate
//! nobody could ever look at; and the overlays are excluded because a plate of an element plus a
//! debugging overlay is a plate of the overlay.
//!
//! # The tolerance, and why it is tighter than the oracle's own
//!
//! R10's hand-off 7, measured rather than predicted: *a page-fraction tolerance is useless at your
//! scale.* A word displaced eight points is 0.85 % of a page and drags the mean similarity only
//! from 1.00 to 0.98 — inside what a cross-producer comparison allows. **A selection handle is a
//! far smaller fraction of a canvas than a word is of that page.** So [`PLATE_TOLERANCE`] halves
//! the differing fraction and raises the worst-window floor, and the worst-window number is the one
//! doing the work: it has no denominator, so one 8 × 8 window that disagrees answers for itself.

use mjx_render_oracle::authority::ReferenceProvider;
use mjx_render_oracle::baseline::{
    self, Approval, ApprovalState, Approver, Baselines, RegenerationIntent, ARTEFACTS,
};
use mjx_render_oracle::digest::sha256_hex;
use mjx_render_oracle::perceptual::{self, Tolerance};
use mjx_render_oracle::plate::{Plate, PlateSet};
use mjx_render_oracle::png;
use mjx_render_oracle::snapshot::{self, Snapshot};
use mjx_tokens::Tokens;

use crate::inventory::{Entry, INVENTORY};
use crate::render::{self, Overlays, Scene};
use crate::state::State;

/// What a plate's pixel tier may spend before a difference is a finding.
///
/// See this module's own documentation. Half of [`Tolerance::EXACT_RASTER`]'s differing fraction —
/// five pixels of a 400 × 267 frame — and a worst-window floor of 0.995 rather than 0.99, because
/// the smallest thing on any of these plates is a note-indicator triangle three points on a side and
/// a tolerance that could absorb one is a tolerance that could absorb the element.
pub const PLATE_TOLERANCE: Tolerance = Tolerance {
    channel: perceptual::CHANNEL_TOLERANCE,
    differing_fraction: 0.000_05,
    structural_similarity: 0.999,
    worst_window_similarity: 0.995,
};

/// The committed baselines, `crates/mjx-canvas-harness/baselines/`.
///
/// A store of its own rather than a corner of the oracle's: the two sets expire for different
/// reasons — a change to `mjx-tokens` moves every plate here and none there — and one directory
/// whose approvals expire in two unrelated ways is a directory nobody re-approves.
#[must_use]
pub fn baselines() -> Baselines {
    Baselines::at(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("baselines"))
}

/// Every entry's stable plate name, in inventory order.
#[must_use]
pub fn names() -> Vec<String> {
    INVENTORY.iter().map(Entry::slug).collect()
}

/// The three artefacts one entry's baseline holds, in [`ARTEFACTS`] order.
///
/// # Errors
///
/// Whatever the scene builder, the painter or the encoder fails with, as a sentence.
pub fn artefacts(
    entry: &'static Entry,
    tokens: &Tokens,
) -> Result<Vec<(&'static str, Vec<u8>)>, String> {
    let scene = Scene::build(entry, tokens, State::CANONICAL);
    let (rendered, bytes) = render::render_png(&scene, Overlays::NONE)?;
    Ok(vec![
        (
            ARTEFACTS[0],
            snapshot::fragments(&rendered.tree).to_text().into_bytes(),
        ),
        (
            ARTEFACTS[1],
            snapshot::commands(&rendered.list).to_text().into_bytes(),
        ),
        (ARTEFACTS[2], bytes),
    ])
}

/// What one entry's three tiers concluded against its committed baseline.
#[derive(Clone, PartialEq, Debug)]
pub struct Verdict {
    /// The entry's plate name.
    pub name: String,
    /// The inventory number.
    pub number: u8,
    /// The first line at which the fragment-tree snapshot differs, or `None`.
    pub fragments: Option<String>,
    /// The same for the display-list snapshot.
    pub commands: Option<String>,
    /// What the pixel comparison concluded, or `None` when it agreed.
    pub pixels: Option<String>,
}

impl Verdict {
    /// Whether all three tiers agreed.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.fragments.is_none() && self.commands.is_none() && self.pixels.is_none()
    }

    /// Which stage moved, in one word — the localisation R10's `tiers::Localisation` gives, stated
    /// here for a set whose tier-one and tier-two subjects are built rather than laid out.
    #[must_use]
    pub fn localisation(&self) -> &'static str {
        match (
            self.fragments.is_some(),
            self.commands.is_some(),
            self.pixels.is_some(),
        ) {
            (false, false, false) => "nothing moved",
            (true, _, _) => "the scene's own geometry",
            (false, true, _) => "the paint or the scene builder",
            (false, false, true) => "the painter",
        }
    }
}

/// Compare one entry against its committed baseline, at all three tiers.
///
/// # Errors
///
/// A sentence, when the baseline cannot be **used** — no approval, a stale digest, a missing
/// artefact. An unusable baseline is an error and never a verdict, for
/// [`mjx_render_oracle::compare_against_baseline`]'s reason: comparing against a baseline nobody
/// approved is the tautology the whole approval machinery exists to refuse, and reporting it as
/// *"the tiers agreed"* would be that tautology with a green tick on it.
pub fn check_entry(
    store: &Baselines,
    entry: &'static Entry,
    tokens: &Tokens,
) -> Result<Verdict, String> {
    let name = entry.slug();
    if let Some(refusal) = store.approval(&name).refusal() {
        return Err(format!("`{name}`: {refusal}"));
    }
    let taken = artefacts(entry, tokens)?;

    let stored_fragments = Snapshot::parse(&String::from_utf8_lossy(
        &store.artefact(&name, ARTEFACTS[0])?,
    ));
    let stored_commands = Snapshot::parse(&String::from_utf8_lossy(
        &store.artefact(&name, ARTEFACTS[1])?,
    ));
    let stored_image = png::decode(&store.artefact(&name, ARTEFACTS[2])?)?;

    let taken_fragments = Snapshot::parse(&String::from_utf8_lossy(&taken[0].1));
    let taken_commands = Snapshot::parse(&String::from_utf8_lossy(&taken[1].1));
    let taken_image = png::decode(&taken[2].1)?;

    let difference = perceptual::compare(&taken_image, &stored_image, PLATE_TOLERANCE)?;
    let verdict = difference.verdict(PLATE_TOLERANCE, true);
    Ok(Verdict {
        name,
        number: entry.number,
        fragments: taken_fragments.first_difference(&stored_fragments),
        commands: taken_commands.first_difference(&stored_commands),
        pixels: if verdict.is_pass() {
            None
        } else {
            Some(verdict.to_string())
        },
    })
}

/// Compare every entry.
///
/// # Errors
///
/// The first unusable baseline, as a sentence.
pub fn check(store: &Baselines, tokens: &Tokens) -> Result<Vec<Verdict>, String> {
    INVENTORY
        .iter()
        .map(|entry| check_entry(store, entry, tokens))
        .collect()
}

/// Write every entry's baseline artefacts, **removing the approval beside each one**.
///
/// # Errors
///
/// Whatever a render or a write fails with, as a sentence.
pub fn regenerate(
    store: &Baselines,
    intent: RegenerationIntent,
    tokens: &Tokens,
    only: Option<u8>,
) -> Result<Vec<String>, String> {
    let mut written = Vec::new();
    for entry in &INVENTORY {
        if only.is_some_and(|number| number != entry.number) {
            continue;
        }
        let name = entry.slug();
        let artefacts = artefacts(entry, tokens)?;
        let owned: Vec<(&str, Vec<u8>)> = artefacts;
        store.regenerate(intent, &name, &owned)?;
        written.push(name);
    }
    if written.is_empty() {
        return Err(format!(
            "no inventory entry has that number; the harness has {} of them, numbered 1 to {}",
            INVENTORY.len(),
            INVENTORY.len()
        ));
    }
    Ok(written)
}

/// Stamp an approval on every baseline that has none.
///
/// **`Approver::Generator`, never `Approver::Human`.** R10's hand-off 9 is explicit and this crate
/// obeys it one level down: an agent cannot manufacture a person's approval, so a generated baseline
/// is recorded as generated, [`Approval::is_reviewed`] stays `false`, and the harness's own page
/// says so at the top. The way a plate becomes human-approved is a person looking at it and running
/// `approve`; nothing else changes it.
///
/// # Errors
///
/// Whatever writing a record fails with, as a sentence.
pub fn stamp_generated(store: &Baselines) -> Result<Vec<String>, String> {
    let mut stamped = Vec::new();
    for entry in &INVENTORY {
        let name = entry.slug();
        if matches!(store.approval(&name), ApprovalState::Approved(_)) {
            continue;
        }
        store.approve(
            &name,
            Approver::Generator,
            "generated by `mjx-canvas-harness regenerate`; NOBODY HAS LOOKED AT THIS IMAGE",
        )?;
        stamped.push(name);
    }
    Ok(stamped)
}

/// Record a **person's** approval of one entry's baseline.
///
/// # Errors
///
/// A sentence, when the number names no entry or the record cannot be written.
pub fn approve(
    store: &Baselines,
    number: u8,
    approver: &str,
    note: &str,
) -> Result<Approval, String> {
    let entry = crate::inventory::entry(number)
        .ok_or_else(|| format!("{number} is not an inventory number; they run from 1 to 61"))?;
    store.approve(
        &entry.slug(),
        Approver::Human {
            name: approver.to_owned(),
        },
        note,
    )
}

/// Every entry whose baseline exists and carries no **human** approval.
#[must_use]
pub fn awaiting_human_review(store: &Baselines) -> Vec<String> {
    let names = names();
    let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
    baseline::awaiting_human_review(store, &borrowed)
}

/// Render every entry and build the manifest R11 and U01 load.
///
/// The `documents` array is **empty**, and that is a statement rather than an omission: this harness
/// needs no document and no fixture, which is exactly what lets in-canvas design be settled before
/// any format renders. The oracle's own plate set fills that array from `mjx-fixtures`; a canvas
/// element has no fixture to name.
///
/// # Errors
///
/// Whatever a render or the encoder fails with, as a sentence.
pub fn plate_set(
    store: &Baselines,
    provider: ReferenceProvider,
    tokens: &Tokens,
) -> Result<PlateSet, String> {
    let mut plates = Vec::with_capacity(INVENTORY.len());
    let mut images = Vec::with_capacity(INVENTORY.len());
    for entry in &INVENTORY {
        let name = entry.slug();
        let scene = Scene::build(entry, tokens, State::CANONICAL);
        let (rendered, bytes) = render::render_png(&scene, Overlays::NONE)?;
        let file = format!("{name}.png");
        let approval = store.approval(&name);

        // The approved image and the amplified difference, **only when there is one**. R10's
        // hand-off 5: a gallery with a black rectangle under every green plate is one a reader
        // learns to scroll past, and by the time a real diff appeared they would scroll past that
        // too.
        let (mut reference_file, mut diff_file, mut difference) = (None, None, None);
        if let Ok(stored) = store
            .artefact(&name, ARTEFACTS[2])
            .and_then(|raw| png::decode(&raw))
        {
            if let Ok(measured) = perceptual::compare(&rendered.image, &stored, PLATE_TOLERANCE) {
                let verdict = measured.verdict(PLATE_TOLERANCE, true);
                if !verdict.is_pass() {
                    let reference = format!("{name}.reference.png");
                    let diff = format!("{name}.diff.png");
                    images.push((reference.clone(), png::encode(&stored)?));
                    images.push((
                        diff.clone(),
                        png::encode(&perceptual::diff_image(&rendered.image, &stored)?)?,
                    ));
                    reference_file = Some(reference);
                    diff_file = Some(diff);
                    difference = Some(verdict.to_string());
                }
            }
        }

        plates.push(Plate {
            name: name.clone(),
            description: format!("{} — {}", entry.title, entry.description),
            file: file.clone(),
            width: rendered.image.width,
            height: rendered.image.height,
            sha256: sha256_hex(&bytes),
            content: entry.content,
            provider,
            exclusion: provider.excludes(entry.content),
            placeholders: rendered.report.placeholders,
            draw_calls: rendered.report.draw_calls,
            covered: rendered.covered,
            approver: match &approval {
                ApprovalState::Approved(approval) => approval.approver.label(),
                other => other.refusal().unwrap_or_else(|| "unapproved".to_owned()),
            },
            reviewed: approval.is_reviewed(),
            // Never true, and computed rather than written: the conjunction of an authoritative
            // provider and an agreement, and there is no authoritative provider. **A canvas overlay
            // has no PowerPoint to be at parity with in the first place** — it is this platform's
            // own design, not a reproduction of somebody else's — which makes the field doubly
            // false here and worth leaving in the manifest to say so.
            parity: provider.is_authoritative() && provider.excludes(entry.content).is_none(),
            reference_file,
            diff_file,
            difference,
        });
        images.push((file, bytes));
    }
    Ok(PlateSet {
        plates,
        documents: Vec::new(),
        images,
        provider,
    })
}
