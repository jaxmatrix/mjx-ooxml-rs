//! Integration tests for MJX-201 P4 — neutralizing inaccessible audio/video media.
//!
//! Media is unmodeled in the library, but every media carrier (`a:videoFile`/`a:audioFile@r:link`, the
//! `a14:media` fallback, `p:snd`/`p:sndTgt` timing sounds) resolves through a media-typed relationship
//! in the slide's `.rels`. These tests drive that relationship layer directly: a media reference is
//! synthesized by adding a `video`/`audio`/`media` relationship to `sample.pptx`'s slide, since the
//! discovery and replacement work on relationships, not on the (absent) media markup.

use std::path::PathBuf;

use mjx_opc::{Package, PartName, Relationship, TargetMode};
use mjx_pptx::{
    default_placeholder_audio, default_placeholder_video, MediaKind, MediaReference, PptxError,
    Presentation,
};

const REL_VIDEO: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/video";
const REL_AUDIO: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio";
const REL_MEDIA: &str = "http://schemas.microsoft.com/office/2007/relationships/media";
const REL_IMAGE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn part(name: &str) -> PartName {
    PartName::new(name).expect("valid part name")
}

fn slide() -> PartName {
    part("/ppt/slides/slide1.xml")
}

fn add_rel(pkg: &mut Package, id: &str, rel_type: &str, target: &str, mode: TargetMode) {
    pkg.add_relationship(
        Some(&slide()),
        Relationship {
            id: id.to_owned(),
            rel_type: rel_type.to_owned(),
            target: target.to_owned(),
            mode,
        },
    )
    .expect("add relationship");
}

/// `sample.pptx` with external `video`, `audio`, and `media` relationships (plus a non-media external
/// image rel that must be ignored) added to slide 1.
fn deck_with_external_media() -> Vec<u8> {
    let mut pkg = Package::open(&fixture("sample.pptx")).expect("open");
    add_rel(
        &mut pkg,
        "rId50",
        REL_VIDEO,
        "https://x/clip.mp4",
        TargetMode::External,
    );
    add_rel(
        &mut pkg,
        "rId51",
        REL_AUDIO,
        "https://x/track.m4a",
        TargetMode::External,
    );
    add_rel(
        &mut pkg,
        "rId52",
        REL_MEDIA,
        "https://x/clip.mp4",
        TargetMode::External,
    );
    add_rel(
        &mut pkg,
        "rId53",
        REL_IMAGE,
        "https://x/poster.png",
        TargetMode::External,
    );
    pkg.save().expect("save")
}

fn media_part_bytes(pkg: &Package, rel_id: &str) -> Vec<u8> {
    let rel = pkg
        .relationships_for(Some(&slide()))
        .expect("slide rels")
        .by_id(rel_id)
        .expect("relationship");
    assert_eq!(
        rel.mode,
        TargetMode::Internal,
        "the media rel must be internalized"
    );
    let resolved = slide().resolve(&rel.target).expect("resolve target");
    pkg.part_bytes(&resolved)
        .expect("placeholder part")
        .to_vec()
}

#[test]
fn default_placeholder_audio_is_a_valid_empty_wav() {
    let wav = default_placeholder_audio();
    assert_eq!(wav.len(), 44);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(
        u32::from_le_bytes([wav[4], wav[5], wav[6], wav[7]]) as usize,
        wav.len() - 8,
        "RIFF chunk size covers the rest of the file"
    );
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(&wav[12..16], b"fmt ");
    assert_eq!(&wav[36..40], b"data");
    assert_eq!(
        u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]),
        0,
        "the data chunk is empty"
    );
}

#[test]
fn default_placeholder_video_is_a_structurally_valid_mp4() {
    let mp4 = default_placeholder_video();
    // Walk the top-level box tree: sizes must tile the file exactly, starting with ftyp then moov.
    let mut boxes = Vec::new();
    let mut i = 0;
    while i + 8 <= mp4.len() {
        let size = u32::from_be_bytes([mp4[i], mp4[i + 1], mp4[i + 2], mp4[i + 3]]) as usize;
        assert!(size >= 8 && i + size <= mp4.len(), "box size in range");
        boxes.push((mp4[i + 4..i + 8].to_vec(), size));
        i += size;
    }
    assert_eq!(i, mp4.len(), "boxes tile the whole file");
    assert_eq!(boxes.len(), 2);
    assert_eq!(boxes[0].0, b"ftyp");
    assert_eq!(boxes[1].0, b"moov");
    assert_eq!(&mp4[8..12], b"isom", "ftyp major brand");
    assert!(
        mp4.windows(4).any(|w| w == b"vide"),
        "the track declares a video handler"
    );
}

#[test]
fn media_references_lists_the_media_rels_with_their_kinds() {
    let mut pres = Presentation::open(&deck_with_external_media()).expect("open");
    assert_eq!(
        pres.media_references(0).expect("media references"),
        vec![
            MediaReference {
                rel_id: "rId50".to_owned(),
                kind: MediaKind::Video,
                target: "https://x/clip.mp4".to_owned(),
                external: true,
            },
            MediaReference {
                rel_id: "rId51".to_owned(),
                kind: MediaKind::Audio,
                target: "https://x/track.m4a".to_owned(),
                external: true,
            },
            MediaReference {
                rel_id: "rId52".to_owned(),
                kind: MediaKind::Media,
                target: "https://x/clip.mp4".to_owned(),
                external: true,
            },
        ],
        "only the audio/video/media rels are reported — the image rel is not media"
    );
}

#[test]
fn replacing_a_video_reference_embeds_a_valid_mp4() {
    let mut pres = Presentation::open(&deck_with_external_media()).expect("open");
    pres.replace_media_with_placeholder(0, "rId50", None)
        .expect("replace video");

    // No longer external, and the reference resolves to the default MP4 placeholder.
    assert!(pres
        .media_references(0)
        .expect("media references")
        .iter()
        .find(|m| m.rel_id == "rId50")
        .is_some_and(|m| !m.external));
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(media_part_bytes(&pkg, "rId50"), default_placeholder_video());
    assert_eq!(
        pkg.content_type_of(&part("/ppt/media/media1.mp4")),
        Some("video/mp4")
    );
}

#[test]
fn replacing_an_audio_reference_embeds_a_valid_wav() {
    let mut pres = Presentation::open(&deck_with_external_media()).expect("open");
    pres.replace_media_with_placeholder(0, "rId51", None)
        .expect("replace audio");
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(media_part_bytes(&pkg, "rId51"), default_placeholder_audio());
    assert_eq!(
        pkg.content_type_of(&part("/ppt/media/media1.wav")),
        Some("audio/x-wav")
    );
}

#[test]
fn replacing_media_can_use_caller_supplied_bytes() {
    let mut pres = Presentation::open(&deck_with_external_media()).expect("open");
    let custom = b"my own media bytes".to_vec();
    pres.replace_media_with_placeholder(0, "rId50", Some(&custom))
        .expect("replace");
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(media_part_bytes(&pkg, "rId50"), custom);
}

#[test]
fn replacing_an_unknown_relationship_is_rejected() {
    let mut pres = Presentation::open(&deck_with_external_media()).expect("open");
    // A non-media rel (the image) and a missing id both fail.
    assert!(matches!(
        pres.replace_media_with_placeholder(0, "rId53", None),
        Err(PptxError::NotAMediaReference { .. })
    ));
    assert!(matches!(
        pres.replace_media_with_placeholder(0, "rId999", None),
        Err(PptxError::NotAMediaReference { .. })
    ));
}

#[test]
fn replacing_an_embedded_media_reference_leaves_the_old_part_sweepable() {
    // An embedded (internal) media part that the caller decides to neutralize anyway.
    let mut pkg = Package::open(&fixture("sample.pptx")).expect("open");
    let old = part("/ppt/media/audio1.wav");
    pkg.insert_part(&old, "audio/x-wav", b"old audio".to_vec())
        .expect("insert");
    add_rel(
        &mut pkg,
        "rId60",
        REL_AUDIO,
        "../media/audio1.wav",
        TargetMode::Internal,
    );
    let mut pres = Presentation::open(&pkg.save().expect("save")).expect("reopen");

    pres.replace_media_with_placeholder(0, "rId60", None)
        .expect("replace");

    let mut swept = Package::open(&pres.save().expect("save")).expect("reopen");
    let removed = swept.remove_unreferenced_parts().expect("sweep");
    assert!(
        removed
            .iter()
            .any(|p| p.as_str() == "/ppt/media/audio1.wav"),
        "the replaced embedded media should be swept: {removed:?}"
    );
}
