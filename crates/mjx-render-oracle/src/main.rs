//! The oracle's own command line: **the explicit paths a person takes, and the ones a test cannot.**
//!
//! Four verbs, and the division between them is the point of the crate:
//!
//! * `check` — compare every specimen against its committed baseline, at all three tiers. What the
//!   suite does, and what a person runs to see the table.
//! * `gallery` — render the plates and write the gallery and the manifest. **The surface a person
//!   approves from**, and the artefact continuous integration attaches.
//! * `regenerate` — rewrite the baselines and *remove their approvals*. Refuses without
//!   `MJX_ORACLE_REGENERATE=1`, because a regenerate path that runs on its own silently ratifies
//!   every regression it exists to catch.
//! * `approve` — record that a **person** looked. Refuses without `MJX_ORACLE_APPROVED_BY`, and
//!   refuses outright under continuous integration, where there is nobody to look.

use std::path::PathBuf;

use mjx_render_oracle::authority::ReferenceProvider;
use mjx_render_oracle::baseline::{
    Approver, Baselines, RegenerationIntent, APPROVED_BY, REGENERATE,
};
use mjx_render_oracle::specimen::{Perturbation, SPECIMENS};
use mjx_render_oracle::{baseline, gallery, plate, png, snapshot, specimen};

fn main() -> std::process::ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let verb = arguments.first().map(String::as_str).unwrap_or("check");
    let result = match verb {
        "check" => check(),
        "gallery" => write_gallery(arguments.get(1).map(PathBuf::from)),
        "regenerate" => regenerate(arguments.get(1).map(String::as_str)),
        "approve" => approve(arguments.get(1).map(String::as_str), arguments.get(2)),
        "list" => list(),
        other => Err(format!(
            "`{other}` is not one of: check, gallery, regenerate, approve, list"
        )),
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("mjx-render-oracle: {reason}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Compare every specimen against its baseline and print the table.
fn check() -> Result<(), String> {
    let baselines = Baselines::committed();
    let mut failed = 0usize;
    println!(
        "{:<18} {:<9} {:<9} {:<9} localisation",
        "specimen", "fragments", "commands", "pixels"
    );
    for specimen in SPECIMENS {
        match mjx_render_oracle::compare_against_baseline(&baselines, specimen, Perturbation::None)
        {
            Ok(comparison) => {
                println!("{}", comparison.row());
                if !comparison.agreed() {
                    failed += 1;
                    println!("{comparison}");
                }
            }
            Err(reason) => {
                failed += 1;
                println!("{:<18} {reason}", specimen.name);
            }
        }
    }
    let awaiting = baseline::awaiting_human_review(
        &baselines,
        &SPECIMENS.iter().map(|s| s.name).collect::<Vec<_>>(),
    );
    if !awaiting.is_empty() {
        println!(
            "\n{} of {} baselines have no human approval: {}",
            awaiting.len(),
            SPECIMENS.len(),
            awaiting.join(", ")
        );
    }
    if failed > 0 {
        return Err(format!("{failed} specimens did not match their baseline"));
    }
    Ok(())
}

/// Render the plates, the manifest and the gallery into `directory`.
fn write_gallery(directory: Option<PathBuf>) -> Result<(), String> {
    let directory = directory.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/oracle-gallery")
    });
    let baselines = Baselines::committed();
    let set = plate::generate(ReferenceProvider::None, &baselines)?;
    set.write(&directory)?;
    let page = directory.join(gallery::GALLERY_FILE);
    std::fs::write(&page, gallery::render(&set))
        .map_err(|error| format!("writing {}: {error}", page.display()))?;
    println!("{}", page.display());
    Ok(())
}

/// Rewrite one specimen's baseline, or every one, removing their approvals.
fn regenerate(which: Option<&str>) -> Result<(), String> {
    let intent = RegenerationIntent::from_environment()?;
    let baselines = Baselines::committed();
    for spec in SPECIMENS {
        if which.is_some_and(|name| name != spec.name) {
            continue;
        }
        let tree = specimen::fragments(spec, Perturbation::None);
        let list = specimen::display_list(spec, Perturbation::None)?;
        let rendered = specimen::render(spec, Perturbation::None)?;
        if rendered.report.placeholders > 0 {
            return Err(format!(
                "`{}` drew {} stand-in shapes; a baseline of a stand-in is a baseline of the \
                 placeholder rather than of the document",
                spec.name, rendered.report.placeholders
            ));
        }
        let image = png::image_from_pixels(&rendered.pixels);
        baselines.regenerate(
            intent,
            spec.name,
            &[
                (
                    baseline::ARTEFACTS[0],
                    snapshot::fragments(&tree).to_text().into_bytes(),
                ),
                (
                    baseline::ARTEFACTS[1],
                    snapshot::commands(&list).to_text().into_bytes(),
                ),
                (baseline::ARTEFACTS[2], png::encode(&image)?),
            ],
        )?;
        // Stamped, not reviewed. The distinction is the whole of `crate::baseline`'s honesty: this
        // is a real approval record, so a later regeneration is caught by its digests, and it says
        // in its own `approver` field that nobody has looked.
        baselines.approve(
            spec.name,
            Approver::Generator,
            "generated by `mjx-render-oracle regenerate`; no human has looked at this image",
        )?;
        println!("regenerated {} (unreviewed)", spec.name);
    }
    Ok(())
}

/// Record that a person approved a baseline.
fn approve(which: Option<&str>, note: Option<&String>) -> Result<(), String> {
    let name = which.ok_or_else(|| {
        format!(
            "which specimen? One of: {}",
            SPECIMENS
                .iter()
                .map(|s| s.name)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    let note = note.ok_or("say why, in a sentence: `approve <specimen> \"why\"`")?;
    if std::env::var("CI").is_ok() {
        return Err(
            "this is running under continuous integration, where there is nobody to look at the \
             image. An approval recorded here would be exactly the tautology the approval exists \
             to break."
                .to_owned(),
        );
    }
    let who = std::env::var(APPROVED_BY).map_err(|_| {
        format!(
            "set {APPROVED_BY} to your own name. An approval that cannot say who gave it is not \
             one — and it is recorded in the file, so use the name you would sign a review with."
        )
    })?;
    if who.trim().is_empty() {
        return Err(format!("{APPROVED_BY} is empty"));
    }
    let approval = Baselines::committed().approve(name, Approver::Human { name: who }, note)?;
    println!(
        "{} approved by {} on {}",
        approval.specimen,
        approval.approver.label(),
        approval.date()
    );
    Ok(())
}

/// Print what is here and what state it is in.
fn list() -> Result<(), String> {
    let baselines = Baselines::committed();
    println!("baselines: {}", baselines.root().display());
    println!("regenerate with {REGENERATE}=1; approve with {APPROVED_BY} set\n");
    for specimen in SPECIMENS {
        let approval = baselines.approval(specimen.name);
        println!(
            "{:<18} {:<14} {}",
            specimen.name,
            specimen.content.label(),
            approval
                .refusal()
                .unwrap_or_else(|| if approval.is_reviewed() {
                    "approved by a person".to_owned()
                } else {
                    "approved by the generator — nobody has looked".to_owned()
                })
        );
    }
    Ok(())
}
