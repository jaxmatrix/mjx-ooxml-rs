//! The plate generator: an arbitrary scene through the software painter into a PNG, plus a manifest
//! a loader can read.
//!
//! # Who consumes this, and why the shape is published rather than implied
//!
//! R11's canvas harness and U01's Storybook gallery both load these plates, and neither of them is
//! Rust. So the contract is a **file layout and a JSON manifest**, stated here, with a reader in
//! `tests/the_plate_manifest_is_loadable.rs` that asks it every question a loader will ask. A
//! manifest checked by asserting on the string that produced it proves that `format!` works; one
//! checked by parsing it back proves it reaches somebody.
//!
//! ```text
//! <output>/
//!   plates.json          the manifest
//!   <specimen>.png       one plate per specimen, straight alpha, 8-bit RGBA
//!   <specimen>.diff.png  present only where a comparison ran and differed
//! ```
//!
//! # ⚠ What every plate carries, and why each field is load-bearing
//!
//! * `placeholders` — **`DrawReport::placeholders`, asserted rather than labelled.** Until Phase G
//!   landed this number was non-zero for every preset shape in the workspace, and a gallery that
//!   said *"these shapes are stand-ins"* in prose would go on saying it after they stopped being
//!   stand-ins, or stop saying it before. It is zero for every plate here and
//!   `tests/every_plate_draws_the_document_and_not_a_stand_in.rs` is what makes that a gate.
//! * `drawCalls` and `covered` — beside it, because **zero placeholders is also true of a page that
//!   drew nothing.** MJXOFF-206's finding, one level up.
//! * `provider`, `authority` and `excluded` — where the reference came from and whether it may
//!   speak about this content at all. A gallery that showed a gradient plate beside a LibreOffice
//!   reference without saying the comparison is not evidence would be inviting exactly the mistake
//!   the user's constraint is about.
//! * `approver` and `reviewed` — whether a **person** has looked at this image. See
//!   [`crate::baseline`].
//! * `parity` — always `false` today, and computed rather than written: it is the conjunction of an
//!   authoritative provider and an agreement, and there is no authoritative provider.

use crate::authority::{ReferenceProvider, RenderedContent};
use crate::baseline::{ApprovalState, Baselines};
use crate::digest::sha256_hex;
use crate::json::quote;
use crate::png::{encode, image_from_pixels, Image};
use crate::specimen::{Specimen, SPECIMENS};

/// The manifest's schema version. A loader that does not recognise it should refuse rather than
/// guess.
pub const MANIFEST_VERSION: u32 = 1;

/// The manifest's file name inside the output directory.
pub const MANIFEST_FILE: &str = "plates.json";

/// One rendered plate and everything a loader has to know about it.
#[derive(Clone, PartialEq, Debug)]
pub struct Plate {
    /// The specimen's stable name.
    pub name: String,
    /// What the page is for.
    pub description: String,
    /// The PNG's file name inside the output directory.
    pub file: String,
    /// Pixels across.
    pub width: u32,
    /// Pixels down.
    pub height: u32,
    /// The PNG's SHA-256, so a loader can cache and a person can check.
    pub sha256: String,
    /// What kind of drawing it is.
    pub content: RenderedContent,
    /// Where its reference came from.
    pub provider: ReferenceProvider,
    /// Why the provider cannot speak about this content, if it cannot.
    pub exclusion: Option<&'static str>,
    /// How many draws used stand-in geometry. **Zero, asserted.**
    pub placeholders: usize,
    /// How many draw calls the page issued.
    pub draw_calls: usize,
    /// How many pixels are not fully transparent.
    pub covered: usize,
    /// Who approved the baseline this plate was rendered against.
    pub approver: String,
    /// Whether a **person** approved it.
    pub reviewed: bool,
    /// Whether this may be described as matching PowerPoint. Always `false` today.
    pub parity: bool,
    /// The approved baseline's file name, and the amplified difference between it and this plate —
    /// **present only when the two differ**.
    ///
    /// # Why they are absent on a green run rather than always written
    ///
    /// When the render matches the baseline they are the same picture, and a gallery that showed it
    /// three times would be a gallery a reader learns to scroll past. When they differ, the plate is
    /// what the renderer draws *now*, the reference is what a person approved, and the diff is where
    /// to look — and MJXOFF-165's constraint is that **a failure a reviewer cannot see is a failure
    /// nobody fixes**. So the three appear together exactly when there is something to see.
    pub reference_file: Option<String>,
    /// The diff's file name, when there is one.
    pub diff_file: Option<String>,
    /// What the pixel comparison against the approved baseline concluded, in one sentence, or
    /// `None` when it agreed.
    pub difference: Option<String>,
}

/// One committed document fixture, and what this oracle can say about it — which today is nothing.
///
/// The rows come from [`mjx_fixtures::package_fixtures`] rather than from a list written here, for
/// `CLAUDE.md`'s stated reason: *a test suite reads its fixture corpus from `mjx-fixtures` — never
/// from a `const FIXTURES` list*. The practical consequence is that the day a format renders, every
/// fixture already has a row waiting for it, and a fixture added in the meantime cannot be missed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DocumentRow {
    /// The fixture's file name.
    pub fixture: String,
    /// `pptx`, `docx` or `xlsx`.
    pub format: String,
    /// What the oracle concluded — `"excluded"` for every row today.
    pub verdict: String,
    /// Why, in a sentence.
    pub reason: String,
}

/// The reason no committed document fixture has a rendered plate yet.
///
/// Stated once, in code, because it is the single most misreadable thing about this gallery: a row
/// with no image looks like a failure, and it is not — it is a stage of the project. Written as a
/// sentence a reader can act on rather than as an empty cell.
pub const NO_FORMAT_RENDERS_YET: &str =
    "no format renders yet. A `.pptx` does become a fragment tree from R14 (MJXOFF-169) onward, and \
     `mjx-layout-pptx` holds that crate's own fragment-tier baselines — but this gallery is the \
     *rendering* path, and turning a document's fragment tree into a display list is R15. This \
     crate also names no format crate at all, by design, so the picture cannot be taken here even \
     once one exists. The row exists so that the day it can be, the fixture is already in the \
     gallery rather than waiting to be remembered.";

/// Everything one run of the generator produced.
#[derive(Clone, PartialEq, Debug)]
pub struct PlateSet {
    /// The plates.
    pub plates: Vec<Plate>,
    /// One row per committed document fixture.
    pub documents: Vec<DocumentRow>,
    /// Each plate's file name and its PNG bytes.
    pub images: Vec<(String, Vec<u8>)>,
    /// Which provider the run was made against.
    pub provider: ReferenceProvider,
}

impl PlateSet {
    /// The manifest, as JSON.
    #[must_use]
    pub fn manifest(&self) -> String {
        let mut out = String::with_capacity(4096);
        out.push_str("{\n");
        out.push_str(&format!("  \"version\": {MANIFEST_VERSION},\n"));
        out.push_str(&format!(
            "  \"generator\": {},\n",
            quote(&format!("mjx-render-oracle {}", env!("CARGO_PKG_VERSION")))
        ));
        // Stated in the manifest because a consumer decoding these files has to know, and because
        // the decision is easier to get wrong than to state. See `crate::png`.
        out.push_str("  \"premultiplied\": false,\n");
        out.push_str(&format!(
            "  \"provider\": {},\n",
            quote(self.provider.label())
        ));
        out.push_str(&format!(
            "  \"authoritative\": {},\n",
            self.provider.is_authoritative()
        ));
        out.push_str(&format!(
            "  \"parityClaimed\": {},\n",
            self.plates.iter().any(|plate| plate.parity)
        ));
        out.push_str("  \"plates\": [\n");
        let plates: Vec<String> = self.plates.iter().map(plate_object).collect();
        out.push_str(&plates.join(",\n"));
        out.push_str("\n  ],\n");
        out.push_str("  \"documents\": [\n");
        let documents: Vec<String> = self.documents.iter().map(document_object).collect();
        out.push_str(&documents.join(",\n"));
        out.push_str("\n  ]\n}\n");
        out
    }

    /// Write the manifest and every PNG into `directory`.
    ///
    /// # Errors
    ///
    /// A sentence naming the path that could not be written.
    pub fn write(&self, directory: &std::path::Path) -> Result<(), String> {
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("creating {}: {error}", directory.display()))?;
        for (name, bytes) in &self.images {
            let path = directory.join(name);
            std::fs::write(&path, bytes)
                .map_err(|error| format!("writing {}: {error}", path.display()))?;
        }
        let manifest = directory.join(MANIFEST_FILE);
        std::fs::write(&manifest, self.manifest())
            .map_err(|error| format!("writing {}: {error}", manifest.display()))
    }
}

/// One plate as a JSON object.
fn plate_object(plate: &Plate) -> String {
    format!(
        "    {{\n      \
         \"name\": {},\n      \
         \"description\": {},\n      \
         \"file\": {},\n      \
         \"width\": {},\n      \
         \"height\": {},\n      \
         \"sha256\": {},\n      \
         \"content\": {},\n      \
         \"provider\": {},\n      \
         \"excluded\": {},\n      \
         \"placeholders\": {},\n      \
         \"drawCalls\": {},\n      \
         \"covered\": {},\n      \
         \"approver\": {},\n      \
         \"reviewed\": {},\n      \
         \"parity\": {},\n      \
         \"referenceFile\": {},\n      \
         \"diffFile\": {},\n      \
         \"difference\": {}\n    }}",
        quote(&plate.name),
        quote(&plate.description),
        quote(&plate.file),
        plate.width,
        plate.height,
        quote(&plate.sha256),
        quote(plate.content.label()),
        quote(plate.provider.label()),
        plate.exclusion.map_or_else(|| "null".to_owned(), quote),
        plate.placeholders,
        plate.draw_calls,
        plate.covered,
        quote(&plate.approver),
        plate.reviewed,
        plate.parity,
        plate
            .reference_file
            .as_deref()
            .map_or_else(|| "null".to_owned(), quote),
        plate
            .diff_file
            .as_deref()
            .map_or_else(|| "null".to_owned(), quote),
        plate
            .difference
            .as_deref()
            .map_or_else(|| "null".to_owned(), quote),
    )
}

/// One document row as a JSON object.
fn document_object(row: &DocumentRow) -> String {
    format!(
        "    {{\n      \
         \"fixture\": {},\n      \
         \"format\": {},\n      \
         \"verdict\": {},\n      \
         \"reason\": {}\n    }}",
        quote(&row.fixture),
        quote(&row.format),
        quote(&row.verdict),
        quote(&row.reason),
    )
}

/// Render `specimen` and encode it.
///
/// # Errors
///
/// Whatever the painter or the encoder fails with, as a sentence.
pub fn render_plate(
    specimen: &Specimen,
) -> Result<(Image, Vec<u8>, mjx_paint::DrawReport), String> {
    let rendered = crate::specimen::render(specimen, crate::specimen::Perturbation::None)?;
    let image = image_from_pixels(&rendered.pixels);
    let bytes = encode(&image)?;
    Ok((image, bytes, rendered.report))
}

/// Generate every plate, against `provider`, reading approvals from `baselines`.
///
/// # Errors
///
/// Whatever a painter or the encoder fails with, as a sentence.
pub fn generate(provider: ReferenceProvider, baselines: &Baselines) -> Result<PlateSet, String> {
    let mut plates = Vec::with_capacity(SPECIMENS.len());
    let mut images = Vec::with_capacity(SPECIMENS.len());
    for specimen in SPECIMENS {
        let (image, bytes, report) = render_plate(specimen)?;
        let file = format!("{}.png", specimen.name);
        let approval = baselines.approval(specimen.name);

        // The approved baseline beside it, and the difference, **only when there is one**. A
        // baseline that cannot be read at all is not an error here: the gallery's whole job is to be
        // lookable-at when something is wrong, and refusing to render because a baseline is missing
        // would take the picture away exactly when it is needed.
        let (mut reference_file, mut diff_file, mut difference) = (None, None, None);
        if let Ok(stored) = baselines
            .artefact(specimen.name, crate::baseline::ARTEFACTS[2])
            .and_then(|raw| crate::png::decode(&raw))
        {
            if let Ok(measured) = crate::perceptual::compare(&image, &stored, specimen.tolerance) {
                let verdict = measured.verdict(specimen.tolerance, true);
                if !verdict.is_pass() {
                    let reference = format!("{}.reference.png", specimen.name);
                    let diff = format!("{}.diff.png", specimen.name);
                    images.push((reference.clone(), encode(&stored)?));
                    images.push((
                        diff.clone(),
                        encode(&crate::perceptual::diff_image(&image, &stored)?)?,
                    ));
                    reference_file = Some(reference);
                    diff_file = Some(diff);
                    difference = Some(verdict.to_string());
                }
            }
        }
        plates.push(Plate {
            name: specimen.name.to_owned(),
            description: specimen.description.to_owned(),
            file: file.clone(),
            width: image.width,
            height: image.height,
            sha256: sha256_hex(&bytes),
            content: specimen.content,
            provider,
            exclusion: provider.excludes(specimen.content),
            placeholders: report.placeholders,
            draw_calls: report.draw_calls,
            covered: image.covered(),
            approver: match &approval {
                ApprovalState::Approved(approval) => approval.approver.label(),
                other => other.refusal().unwrap_or_else(|| "unapproved".to_owned()),
            },
            reviewed: approval.is_reviewed(),
            // Never true today, and computed rather than written. `ReferenceProvider::None` and
            // `LibreOffice` are both non-authoritative, so this is the conjunction that
            // `crate::authority::Baseline::may_be_called_parity` already refuses one level down.
            parity: provider.is_authoritative() && provider.excludes(specimen.content).is_none(),
            reference_file,
            diff_file,
            difference,
        });
        images.push((file, bytes));
    }
    Ok(PlateSet {
        plates,
        documents: document_rows(),
        images,
        provider,
    })
}

/// One row per committed package fixture, derived from `mjx-fixtures`.
#[must_use]
pub fn document_rows() -> Vec<DocumentRow> {
    mjx_fixtures::package_fixtures()
        .into_iter()
        .map(|fixture| {
            let format = std::path::Path::new(&fixture)
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("")
                .to_owned();
            DocumentRow {
                fixture,
                format,
                verdict: "excluded".to_owned(),
                reason: NO_FORMAT_RENDERS_YET.to_owned(),
            }
        })
        .collect()
}
