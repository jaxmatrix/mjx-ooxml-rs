//! **A failure a reviewer cannot see is a failure nobody fixes** — MJXOFF-165's own constraint, and
//! the one that decides whether this harness is used or worked around.
//!
//! A pixel-tier regression is a *picture*. A log line saying `0.83 % of pixels differ, worst at
//! (214, 81)` is true, precise, and useless to the person who has to decide whether the renderer got
//! better or worse. So when a plate stops matching its approved baseline, the gallery carries three
//! images side by side — what the renderer draws now, what a person approved, and the amplified
//! difference — and the manifest names all three so a machine can attach them too.
//!
//! # The part that is easy to get wrong
//!
//! Producing a diff image is easy; producing one **only when it is needed** is the design. A gallery
//! that showed a black rectangle under every green plate would be one a reader learns to scroll
//! past, and by the time a real diff appeared they would scroll past that too. So this file asserts
//! both halves: absent on a run that matches, present and non-black on one that does not.

use std::path::PathBuf;

use mjx_render_oracle::baseline::{Approver, Baselines, ARTEFACTS};
use mjx_render_oracle::json::{parse, Value};
use mjx_render_oracle::plate::{generate, MANIFEST_FILE};
use mjx_render_oracle::specimen::SPECIMENS;
use mjx_render_oracle::{gallery, png, ReferenceProvider};

const SUBJECT: &str = "solid-panels";

#[test]
fn a_matching_run_carries_no_diff_at_all() {
    let set = generate(ReferenceProvider::None, &Baselines::committed()).expect("plates generate");
    for plate in &set.plates {
        assert_eq!(
            plate.diff_file, None,
            "`{}` matches its baseline and carries a diff anyway; a gallery with a diff under every \
             plate is one a reader stops looking at",
            plate.name
        );
        assert_eq!(plate.reference_file, None);
        assert_eq!(plate.difference, None);
    }
    let page = gallery::render(&set);
    assert!(
        !page.contains("does not match the approved baseline"),
        "the gallery reports a regression on a run where every plate matched"
    );
    // Exactly one image per plate, and nothing else.
    assert_eq!(set.images.len(), set.plates.len());
}

#[test]
fn a_regression_carries_the_approved_image_and_the_difference() {
    // A scratch baseline whose stored image is the real one with a block painted over it. Nothing
    // about the renderer changes — only what was approved does — which is the shape of a real
    // regression seen from the harness's side.
    let directory = scratch("regression");
    let real = Baselines::committed();
    std::fs::create_dir_all(directory.join(SUBJECT)).expect("a scratch baseline");
    for specimen in SPECIMENS {
        std::fs::create_dir_all(directory.join(specimen.name)).expect("a directory");
        for artefact in ARTEFACTS {
            std::fs::write(
                directory.join(specimen.name).join(artefact),
                real.artefact(specimen.name, artefact).expect("an artefact"),
            )
            .expect("copying");
        }
    }
    let mut image =
        png::decode(&real.artefact(SUBJECT, ARTEFACTS[2]).expect("the image")).expect("it decodes");
    for y in 30..90u32 {
        for x in 30..150u32 {
            let offset = ((y as usize) * (image.width as usize) + (x as usize)) * 4;
            image.rgba[offset..offset + 4].copy_from_slice(&[0x00, 0xc0, 0x40, 0xff]);
        }
    }
    std::fs::write(
        directory.join(SUBJECT).join(ARTEFACTS[2]),
        png::encode(&image).expect("encoding"),
    )
    .expect("writing the altered baseline");

    let baselines = Baselines::at(&directory);
    for specimen in SPECIMENS {
        baselines
            .approve(specimen.name, Approver::Generator, "a scratch approval")
            .expect("approving");
    }

    let set = generate(ReferenceProvider::None, &baselines).expect("plates generate");
    let plate = set
        .plates
        .iter()
        .find(|plate| plate.name == SUBJECT)
        .expect("the subject is in the set");
    let reference = plate
        .reference_file
        .as_deref()
        .expect("a regressed plate carries the image that was approved");
    let diff = plate
        .diff_file
        .as_deref()
        .expect("a regressed plate carries the difference");
    let difference = plate
        .difference
        .as_deref()
        .expect("a regressed plate says what the comparison concluded");
    assert!(
        difference.contains("differs") && difference.contains("worst at"),
        "the sentence does not name a place: {difference}"
    );

    // And the diff image is **not black**, which is the way this goes hollow: a diff that amplified
    // nothing looks exactly like a pass.
    let written: std::collections::BTreeMap<&str, &Vec<u8>> = set
        .images
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes))
        .collect();
    let diff_image =
        png::decode(written.get(diff).expect("the diff is in the set")).expect("the diff decodes");
    let lit = diff_image
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0)
        .count();
    assert!(
        lit > 1_000,
        "the diff image lights up {lit} pixels for a 120x60 block of solid green; a diff that shows \
         nothing is a diff a reviewer reads as a pass"
    );
    assert!(
        written.contains_key(reference),
        "the approved image is not beside it"
    );

    // The other four plates still match, so the gallery is not simply reporting everything.
    assert_eq!(
        set.plates
            .iter()
            .filter(|plate| plate.diff_file.is_some())
            .count(),
        1,
        "one baseline was altered and more than one plate reports a difference"
    );

    // The gallery shows all three, and the manifest names them so continuous integration can attach
    // them without parsing HTML.
    set.write(&directory).expect("writing");
    let page = gallery::render(&set);
    std::fs::write(directory.join(gallery::GALLERY_FILE), &page).expect("writing the gallery");
    assert!(page.contains("does not match the approved baseline"));
    assert!(page.contains(&format!("src=\"{reference}\"")));
    assert!(page.contains(&format!("src=\"{diff}\"")));
    assert!(page.contains("Does not match its approved baseline: 1."));
    assert!(directory.join(reference).is_file() && directory.join(diff).is_file());

    let manifest = parse(
        &std::fs::read_to_string(directory.join(MANIFEST_FILE)).expect("reading the manifest"),
    )
    .expect("it parses");
    let entry = manifest
        .get("plates")
        .and_then(Value::array)
        .and_then(|plates| {
            plates
                .iter()
                .find(|plate| plate.get("name").and_then(Value::string) == Some(SUBJECT))
        })
        .expect("the subject is in the manifest");
    assert_eq!(
        entry.get("referenceFile").and_then(Value::string),
        Some(reference)
    );
    assert_eq!(entry.get("diffFile").and_then(Value::string), Some(diff));
    assert_eq!(
        entry.get("difference").and_then(Value::string),
        Some(difference)
    );
    // And a plate that matched carries nulls rather than being absent from the schema.
    let matched = manifest
        .get("plates")
        .and_then(Value::array)
        .and_then(|plates| {
            plates
                .iter()
                .find(|plate| plate.get("name").and_then(Value::string) == Some("preset-star"))
        })
        .expect("another plate");
    assert_eq!(matched.get("diffFile"), Some(&Value::Null));

    let _ = std::fs::remove_dir_all(&directory);
}

/// Where a case leaves its artefacts. Named per case, because Cargo runs a binary's cases on several
/// threads at once.
fn scratch(case: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/oracle-scratch")
        .join(case);
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}
