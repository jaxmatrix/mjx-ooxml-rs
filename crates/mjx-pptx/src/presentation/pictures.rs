//! Pictures and media: adding an image, reading or replacing the one a picture shows, and the
//! audio/video and linked-image references a deck carries.

use mjx_dml::PictureFill;
use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawDocument, RawElement, RawNode};
use mjx_ooxml_types::namespaces::{DML_MAIN, PML};
use mjx_opc::{ImageFormat, PartName, Relationship, TargetMode};

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::external::{
    default_placeholder_audio, default_placeholder_video, LinkedImage, MediaKind, MediaReference,
    DEFAULT_PLACEHOLDER_IMAGE,
};
use crate::geometry::ShapeBounds;
use crate::slide::ShapeKind;
use crate::surface::Surface;
use crate::{build, constants, nav, slide};

use super::deck::{dir_of, relationship_prefix, stem_number};
use super::effective::resolve_shape_in;
use super::element_builders::build_sp_pr;
use super::Presentation;

impl Presentation {
    /// Appends a picture (`p:pic`) showing `bytes` to `surface`, laid out at `bounds`.
    /// Returns the index of the new shape in the slide's one shape index space (see
    /// [`shape_count`](Self::shape_count)); [`shape_kind`](Self::shape_kind) reports it as
    /// [`ShapeKind::Picture`], and the whole `p:spPr` surface — outline, effects, geometry — applies
    /// to it like any other shape.
    ///
    /// The image part and its relationship are created by [`add_image`](Self::add_image), so adding
    /// the same picture twice stores the bytes once. The image is stretched to fill `bounds`; since
    /// nothing here decodes the image, its natural size is unknown and the caller chooses the extent
    /// (the emitted `a:picLocks@noChangeAspect` keeps the ratio locked for later interactive resizing).
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range, the bytes match no known image format
    /// ([`UnrecognizedImageFormat`](PptxError::UnrecognizedImageFormat)), the slide is malformed, or a
    /// package edit fails.
    pub fn add_picture(
        &mut self,
        surface: impl Into<Surface>,
        bytes: &[u8],
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        let surface = surface.into();
        // The image part and relationship first: if the bytes are not an image, nothing is edited.
        let rel_id = self.add_image(surface, bytes)?;

        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let rel_declaration = build::relationship_prefix_declaration(root, interner);
        let sp_tree = slide::sp_tree_mut(root, interner)?;

        let next_id = slide::max_cnvpr_id(sp_tree, interner).max(1) + 1;
        let picture = build_picture(interner, next_id, &rel_id, bounds, rel_declaration);
        sp_tree.children.push(RawNode::Element(picture));
        sp_tree.empty = false;

        Ok(slide::shapes(sp_tree, interner).count() - 1)
    }

    /// Every audio/video/media relationship on `surface`, with where each is referenced from and
    /// whether it is external.
    ///
    /// An external media reference is the case that can be unreachable on another platform; a slide
    /// still shows a media object's poster image, so
    /// [`replace_media_with_placeholder`](Self::replace_media_with_placeholder) can neutralize it
    /// safely. Media is reported by relationship id, not shape index: a single media object is
    /// referenced from its `p:pic`, its `a14:media` fallback, and any timing/transition sound, all of
    /// which resolve through these relationships (and timing/transition sounds are not shapes). Reading
    /// does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `surface` cannot be resolved.
    pub fn media_references(
        &mut self,
        surface: impl Into<Surface>,
    ) -> Result<Vec<MediaReference>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let Some(rels) = self.package.relationships_for(Some(&slide_part)) else {
            return Ok(Vec::new());
        };
        Ok(rels
            .iter()
            .filter_map(|rel| {
                media_kind_of(&rel.rel_type).map(|kind| MediaReference {
                    rel_id: rel.id.clone(),
                    kind,
                    target: rel.target.clone(),
                    external: rel.mode == TargetMode::External,
                })
            })
            .collect())
    }

    /// Replaces the media that relationship `rel_id` on `surface` binds with an in-package placeholder,
    /// so a reference to unreachable external audio/video resolves inside the package instead. The
    /// placeholder is `placeholder` if given, else a built-in one matching the media kind — a valid
    /// silent WAV for audio ([`default_placeholder_audio`](crate::default_placeholder_audio)) or a
    /// minimal MP4 for video ([`default_placeholder_video`](crate::default_placeholder_video)). The
    /// relationship is retargeted at the placeholder, so every carrier that named it — the `p:pic`, its
    /// `a14:media` fallback, timing/transition sounds — now resolves locally; the poster image is
    /// untouched.
    ///
    /// The caller decides a reference is inaccessible (the library does no external I/O); use
    /// [`media_references`](Self::media_references) to find the candidates. If the old reference was to
    /// an embedded part, that part is left unreferenced; sweep it with
    /// [`Package::remove_unreferenced_parts`](mjx_opc::Package::remove_unreferenced_parts) if wanted.
    ///
    /// # Errors
    /// [`PptxError::NotAMediaReference`] if `rel_id` names no audio/video/media relationship on the
    /// surface, or another [`PptxError`] if the surface is malformed.
    pub fn replace_media_with_placeholder(
        &mut self,
        surface: impl Into<Surface>,
        rel_id: &str,
        placeholder: Option<&[u8]>,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;

        let kind = self
            .package
            .relationships_for(Some(&slide_part))
            .and_then(|rels| rels.by_id(rel_id))
            .and_then(|rel| media_kind_of(&rel.rel_type))
            .ok_or_else(|| PptxError::NotAMediaReference {
                rel_id: rel_id.to_owned(),
            })?;

        let (extension, content_type, default_bytes): (&str, &str, fn() -> Vec<u8>) = match kind {
            MediaKind::Video => (
                "mp4",
                constants::CONTENT_TYPE_MP4,
                default_placeholder_video,
            ),
            MediaKind::Audio | MediaKind::Media => (
                "wav",
                constants::CONTENT_TYPE_WAV,
                default_placeholder_audio,
            ),
        };
        let bytes = placeholder.map_or_else(default_bytes, <[u8]>::to_vec);

        let placeholder_part = self.next_media_part_stem("media", extension)?;
        self.package
            .insert_part(&placeholder_part, content_type, bytes)?;
        let target = slide_part.relative_target(&placeholder_part);
        self.package.retarget_relationship(
            Some(&slide_part),
            rel_id,
            &target,
            TargetMode::Internal,
        )?;
        Ok(())
    }

    /// Stores `bytes` as an image part and points the existing relationship `rel_id` on `surface` at
    /// it — how a snapshot image is replaced without touching the markup that names it.
    pub(super) fn retarget_image_relationship(
        &mut self,
        surface: Surface,
        rel_id: &str,
        bytes: &[u8],
    ) -> Result<(), PptxError> {
        let format = ImageFormat::sniff(bytes).ok_or(PptxError::UnrecognizedImageFormat)?;
        let media_part = match self.media_part_with_bytes(bytes) {
            Some(existing) => existing,
            None => {
                let part = self.next_media_part(format.file_extension())?;
                self.package
                    .set_content_type_default(format.file_extension(), format.content_type())?;
                self.package
                    .insert_part(&part, format.content_type(), bytes.to_vec())?;
                part
            }
        };
        let slide_part = self.surface_part(surface)?;
        let target = nav::relative_target(&slide_part, &media_part);
        self.package.retarget_relationship(
            Some(&slide_part),
            rel_id,
            &target,
            TargetMode::Internal,
        )?;
        Ok(())
    }

    /// The relationship id that binds picture `shape_idx` on `surface` to its image — whether the blip
    /// *embeds* the image (`p:blipFill > a:blip@r:embed`) or *links* it
    /// (`@r:link`, typically an external file). `None` when the blip binds no image. Prefers the embed
    /// id when both are present. Reading does not dirty the part.
    ///
    /// To tell an embedded image from a linked one, or to read where a linked image points, use
    /// [`picture_image_link_target`](Self::picture_image_link_target): it returns `Some` only for a
    /// link. [`picture_image_bytes`](Self::picture_image_bytes) reads an embedded (or internal-linked)
    /// image's bytes but reports an external link as [`PptxError::ExternalTarget`].
    ///
    /// # Errors
    /// Returns [`PptxError::ShapeIsNotAPicture`] if the shape is not a `p:pic`,
    /// [`PptxError::PictureHasNoImage`] if it is missing its `p:blipFill`, or another
    /// [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn picture_image_rel_id(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<String>, PptxError> {
        let (embed, link) = self.picture_image_rel_ids(surface.into(), &shape_idx.into())?;
        Ok(embed.or(link))
    }

    /// The target of the image that picture `shape_idx` on `surface` *links* (`p:blipFill >
    /// a:blip@r:link`), exactly as the relationship records it — an external path/URL for the common
    /// case, or an in-package part target for an internal link. `None` when the picture embeds its
    /// image (or binds none): an embedded image has no separate target, its bytes are the image.
    ///
    /// This is what makes a linked image *addressable*: [`picture_image_bytes`](Self::picture_image_bytes)
    /// cannot return bytes that live outside the package, but the caller can still learn — and act on —
    /// where the image points. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`picture_image_rel_id`](Self::picture_image_rel_id).
    pub fn picture_image_link_target(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<String>, PptxError> {
        let surface = surface.into();
        let (_embed, link) = self.picture_image_rel_ids(surface, &shape_idx.into())?;
        let Some(link_id) = link else {
            return Ok(None);
        };
        let slide_part = self.surface_part(surface)?;
        let Some(rels) = self.package.relationships_for(Some(&slide_part)) else {
            return Ok(None);
        };
        Ok(rels.by_id(&link_id).map(|rel| rel.target.clone()))
    }

    /// The `(embed, link)` relationship ids a picture's blip carries (`a:blip@r:embed` / `@r:link`),
    /// each `None` when absent. The one place `p:pic > p:blipFill > a:blip` is resolved for reading, so
    /// the embed/link readers stay in lock-step.
    fn picture_image_rel_ids(
        &mut self,
        surface: Surface,
        shape_idx: &ShapePath,
    ) -> Result<(Option<String>, Option<String>), PptxError> {
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let picture = picture_at(sp_tree, &doc.interner, surface, shape_idx)?;
        let blip_fill = nav::child(picture, &doc.interner, PML, "blipFill")
            .ok_or(PptxError::PictureHasNoImage)?;
        let blip_fill = PictureFill::from_xml(blip_fill, &doc.interner)?;
        let embed = blip_fill.image_rel_id(&doc.interner);
        let link = blip_fill.image_link_id(&doc.interner);
        Ok((embed, link))
    }

    /// The stored bytes of the image that picture `shape_idx` on `surface` binds, exactly as the
    /// package holds them (never decoded or re-encoded), or `None` when the picture binds no image.
    /// Borrowed from the package, so a large image is not copied.
    ///
    /// An embedded image (or a link whose target is *inside* the package) resolves to bytes. A picture
    /// that links an **external** image has no bytes here — the image lives outside the package — and
    /// this reports [`PptxError::ExternalTarget`]; use
    /// [`picture_image_link_target`](Self::picture_image_link_target) to read where it points.
    ///
    /// # Errors
    /// As [`picture_image_rel_id`](Self::picture_image_rel_id), plus
    /// [`PptxError::ExternalTarget`] if the relationship points outside the package.
    pub fn picture_image_bytes(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<&[u8]>, PptxError> {
        let surface = surface.into();
        let Some(rel_id) = self.picture_image_rel_id(surface, shape_idx)? else {
            return Ok(None);
        };
        let slide_part = self.surface_part(surface)?;
        let Some(part) = self.part_for_rel(&slide_part, &rel_id)? else {
            return Ok(None);
        };
        Ok(self.package.part_bytes(&part))
    }

    /// Points picture `shape_idx` on `surface` at `bytes`, adding the image to the package if
    /// it is not already there ([`add_image`](Self::add_image), so identical bytes are stored once)
    /// and rewriting the blip's `@r:embed`. Any `@r:link` is dropped — the picture now embeds its
    /// image — and the rest of the `p:blipFill` (source rect, tile/stretch) is preserved.
    ///
    /// The previously embedded image part is **left in the package**: another shape may still show it,
    /// and sweeping unreferenced parts is a package-wide graph operation, not this method's job. An
    /// unreferenced part is legal and simply unused.
    ///
    /// # Errors
    /// As [`picture_image_rel_id`](Self::picture_image_rel_id), plus
    /// [`UnrecognizedImageFormat`](PptxError::UnrecognizedImageFormat) if the bytes match no known
    /// image format.
    pub fn set_picture_image(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        bytes: &[u8],
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        // Validate the shape kind before editing the package, so a wrong index adds no image part.
        {
            let slide_part = self.surface_part(surface)?;
            let doc = self.package.part_tree(&slide_part)?;
            let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
            let picture = picture_at(sp_tree, &doc.interner, surface, &path)?;
            if nav::child(picture, &doc.interner, PML, "blipFill").is_none() {
                return Err(PptxError::PictureHasNoImage);
            }
        }
        let rel_id = self.add_image(surface, bytes)?;

        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let rel_prefix = relationship_prefix(root, interner);
        let picture = resolve_shape_in(root, interner, surface, &path)?;
        slide::set_blip_embed(picture, interner, rel_prefix, &rel_id)
    }

    /// Every picture on `surface` that *links* its image (`a:blip@r:link`) rather than embedding it,
    /// with where each links from — the candidates for
    /// [`replace_linked_image_with_placeholder`](Self::replace_linked_image_with_placeholder). A linked
    /// image is the common source that can be unreachable on another platform; this saves the caller
    /// from walking the shapes themselves. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `surface` cannot be resolved or the slide is malformed.
    pub fn linked_images(
        &mut self,
        surface: impl Into<Surface>,
    ) -> Result<Vec<LinkedImage>, PptxError> {
        let surface = surface.into();
        let count = self.shape_count(surface)?;
        let mut linked = Vec::new();
        for shape_index in 0..count {
            if self.shape_kind(surface, shape_index)? != ShapeKind::Picture {
                continue;
            }
            if let Some(target) = self.picture_image_link_target(surface, shape_index)? {
                linked.push(LinkedImage {
                    shape_index,
                    target,
                });
            }
        }
        Ok(linked)
    }

    /// Replaces the *linked* image of picture `shape_idx` on `surface` with an embedded placeholder,
    /// so a picture that points at an unreachable external file resolves inside the package instead.
    /// The placeholder is `placeholder` if given, else [`DEFAULT_PLACEHOLDER_IMAGE`]. The picture
    /// becomes an ordinary embedded picture (`@r:link` → `@r:embed`), keeping its bounds and the rest
    /// of its `p:blipFill`, and the now-unused link relationship is dropped.
    ///
    /// The caller decides a link is inaccessible (the library does no external I/O); use
    /// [`linked_images`](Self::linked_images) to find the candidates. If the picture *embeds* its image
    /// there is no link to replace and this returns [`PptxError::PictureImageNotLinked`].
    ///
    /// If the old link happened to be *internal*, the part it named may be left unreferenced; sweep it
    /// with [`Package::remove_unreferenced_parts`](mjx_opc::Package::remove_unreferenced_parts) if
    /// wanted. This never removes parts on its own.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAPicture`] if the shape is not a `p:pic`,
    /// [`PptxError::PictureImageNotLinked`] if it embeds rather than links,
    /// [`PptxError::UnrecognizedImageFormat`] if the placeholder bytes match no known image format, or
    /// another [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn replace_linked_image_with_placeholder(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        placeholder: Option<&[u8]>,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let (_embed, link) = self.picture_image_rel_ids(surface, &path)?;
        let Some(link_id) = link else {
            return Err(PptxError::PictureImageNotLinked);
        };
        let bytes = placeholder.unwrap_or(DEFAULT_PLACEHOLDER_IMAGE);
        // Embeds the placeholder and drops the `@r:link` attribute (its relationship still lingers).
        self.set_picture_image(surface, path, bytes)?;
        // Drop the now-dangling link relationship so nothing points outside the package.
        let slide_part = self.surface_part(surface)?;
        self.package
            .remove_relationship(Some(&slide_part), &link_id)?;
        Ok(())
    }

    /// Stores `bytes` as an image part of the package and relates it to `surface`, returning
    /// the **slide-scoped relationship id** that names the image — the `rel_id` to hand to
    /// [`FillSpec::Picture`](mjx_dml::FillSpec::Picture) via [`set_shape_fill`](Self::set_shape_fill).
    ///
    /// The format is identified from the bytes ([`ImageFormat::sniff`]), which decides the media part's
    /// extension and its content type; the bytes themselves are stored verbatim and never re-encoded.
    /// The part is named `media/image{N}.{ext}` beside the presentation part, with `N` one past the
    /// largest existing image number.
    ///
    /// **Identical images are stored once**: if a media part already holds exactly these bytes it is
    /// reused, and if that surface already relates to it, the existing relationship id is returned and
    /// the package is not touched at all. Otherwise only `[Content_Types].xml`, the new media part, and
    /// that part's `.rels` change — every other pre-existing part stays byte-identical.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range,
    /// [`PptxError::UnrecognizedImageFormat`] if the bytes match no known image format, or another
    /// [`PptxError`] if a package edit fails.
    pub fn add_image(
        &mut self,
        surface: impl Into<Surface>,
        bytes: &[u8],
    ) -> Result<String, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let format = ImageFormat::sniff(bytes).ok_or(PptxError::UnrecognizedImageFormat)?;

        let media_part = match self.media_part_with_bytes(bytes) {
            Some(existing) => {
                // Already stored: reuse the slide's relationship to it when there is one.
                if let Some(id) = self.image_rel_id_for(&slide_part, &existing)? {
                    return Ok(id);
                }
                existing
            }
            None => {
                let part = self.next_media_part(format.file_extension())?;
                // Registering the Default first means `insert_part` adds no per-part Override.
                self.package
                    .set_content_type_default(format.file_extension(), format.content_type())?;
                self.package
                    .insert_part(&part, format.content_type(), bytes.to_vec())?;
                part
            }
        };

        let rel_id = self.next_rid_for(&slide_part);
        self.package.add_relationship(
            Some(&slide_part),
            Relationship {
                id: rel_id.clone(),
                rel_type: constants::REL_IMAGE.to_owned(),
                target: nav::relative_target(&slide_part, &media_part),
                mode: TargetMode::Internal,
            },
        )?;
        Ok(rel_id)
    }

    /// The media part whose stored bytes equal `bytes`, if the package already holds one. Comparing
    /// slices short-circuits on length, so this is a cheap scan even for large images.
    fn media_part_with_bytes(&self, bytes: &[u8]) -> Option<PartName> {
        let media_dir = format!("{}media/", dir_of(self.presentation_part.as_str()));
        self.package
            .part_names()
            .filter(|part| part.as_str().starts_with(&media_dir))
            .find(|part| self.package.part_bytes(part) == Some(bytes))
    }

    /// The id of `source`'s existing [`REL_IMAGE`](constants::REL_IMAGE) relationship pointing at
    /// `target`, or `None` if it has none.
    fn image_rel_id_for(
        &self,
        source: &PartName,
        target: &PartName,
    ) -> Result<Option<String>, PptxError> {
        let Some(rels) = self.package.relationships_for(Some(source)) else {
            return Ok(None);
        };
        for rel in rels.by_type(constants::REL_IMAGE) {
            if rel.mode == TargetMode::External {
                continue; // a linked image never names a part in this package
            }
            if &nav::resolve_target(source, &rel.target)? == target {
                return Ok(Some(rel.id.clone()));
            }
        }
        Ok(None)
    }

    /// A fresh image part name in the presentation's `media/` directory: `image{N}.{extension}` with
    /// `N` one past the largest existing image number, whatever its extension.
    fn next_media_part(&self, extension: &str) -> Result<PartName, PptxError> {
        let media_dir = format!("{}media/", dir_of(self.presentation_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = image_number(part.as_str(), &media_dir) {
                max_n = max_n.max(n);
            }
        }
        let name = format!("{media_dir}image{}.{extension}", max_n + 1);
        PartName::new(&name).map_err(PptxError::from)
    }

    /// A fresh media part name `media/{stem}{N}.{extension}`, with `N` one past the largest existing
    /// `{stem}*` media part. Used for placeholder media (`media{N}.wav` / `.mp4`).
    fn next_media_part_stem(&self, stem: &str, extension: &str) -> Result<PartName, PptxError> {
        let media_dir = format!("{}media/", dir_of(self.presentation_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = stem_number(part.as_str(), &media_dir, stem) {
                max_n = max_n.max(n);
            }
        }
        let name = format!("{media_dir}{stem}{}.{extension}", max_n + 1);
        PartName::new(&name).map_err(PptxError::from)
    }

    /// A fresh embedding part name: `embeddings/oleObject{N}.{extension}` beside the presentation part,
    /// with `N` one past the largest existing `oleObject*` embedding number.
    pub(super) fn next_embedding_part(&self, extension: &str) -> Result<PartName, PptxError> {
        let embeddings_dir = format!("{}embeddings/", dir_of(self.presentation_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = embedding_number(part.as_str(), &embeddings_dir) {
                max_n = max_n.max(n);
            }
        }
        let name = format!("{embeddings_dir}oleObject{}.{extension}", max_n + 1);
        PartName::new(&name).map_err(PptxError::from)
    }
}

/// The `p:pic` addressed by `path` in `sp_tree`, or [`PptxError::ShapeIsNotAPicture`] when that
/// address names a shape of another kind (the one index space covers every kind).
pub(super) fn picture_at<'a>(
    sp_tree: &'a RawElement,
    interner: &'a Interner,
    surface: Surface,
    path: &ShapePath,
) -> Result<&'a RawElement, PptxError> {
    let shape = slide::resolve_shape(sp_tree, interner, path).map_err(|count| {
        PptxError::ShapeIndexOutOfRange {
            surface,
            path: path.clone(),
            count,
        }
    })?;
    match slide::shape_kind(shape, interner) {
        Some(ShapeKind::Picture) => Ok(shape),
        _ => Err(PptxError::ShapeIsNotAPicture),
    }
}

/// Extracts `N` from an `image{N}.{ext}` part directly inside `dir` (e.g. `/ppt/media/image3.png`
/// with `dir = /ppt/media/` → `3`), whatever the extension. Returns `None` for anything else.
fn image_number(part: &str, dir: &str) -> Option<u32> {
    let name = part.strip_prefix(dir)?.strip_prefix("image")?;
    let digits = &name[..name.find('.').unwrap_or(name.len())];
    digits.parse::<u32>().ok()
}

/// The `N` in `{dir}oleObject{N}.{ext}`, whatever the extension, or `None` if `part` is not such an
/// embedding.
fn embedding_number(part: &str, dir: &str) -> Option<u32> {
    stem_number(part, dir, "oleObject")
}

/// Classifies a relationship type as an audio/video/media reference, or `None` for anything else.
fn media_kind_of(rel_type: &str) -> Option<MediaKind> {
    match rel_type {
        constants::REL_VIDEO => Some(MediaKind::Video),
        constants::REL_AUDIO => Some(MediaKind::Audio),
        constants::REL_MEDIA => Some(MediaKind::Media),
        _ => None,
    }
}

/// A whole `p:pic` picture: `nvPicPr` (with `a:picLocks@noChangeAspect`) + a `p:blipFill` embedding
/// `rel_id` stretched to the shape + `spPr` with a rectangular geometry at `bounds`.
///
/// `p:blipFill` is a PresentationML element of the DrawingML `CT_BlipFillProperties` type, so it is
/// built here with the `p`-prefixed builders rather than by `mjx_dml::PictureFill::new` (which emits
/// `a:blipFill`); reading it back does reuse `PictureFill`, whose fidelity wrapper is name-agnostic.
/// `rel_declaration` is an `xmlns:r` declaration for the `r:embed` prefix when the slide does not
/// already bind it (see [`build::relationship_prefix_declaration`]).
pub(super) fn build_picture(
    interner: &mut Interner,
    id: u32,
    rel_id: &str,
    bounds: ShapeBounds,
    rel_declaration: Option<RawAttribute>,
) -> RawElement {
    // p:nvPicPr — cNvPr, cNvPicPr (locking the aspect ratio, as Office writes it), and an empty nvPr.
    let cnvpr_attrs = vec![
        build::attr(interner, "id", &id.to_string()),
        build::attr(interner, "name", &format!("Picture {id}")),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
    let lock_attrs = vec![build::attr(interner, "noChangeAspect", "1")];
    let pic_locks = build::leaf(interner, "a", DML_MAIN, "picLocks", lock_attrs);
    let c_nv_pic_pr = build::node(
        interner,
        "p",
        PML,
        "cNvPicPr",
        Vec::new(),
        vec![RawNode::Element(pic_locks)],
    );
    let nv_pr = build::leaf(interner, "p", PML, "nvPr", Vec::new());
    let nv_pic_pr = build::node(
        interner,
        "p",
        PML,
        "nvPicPr",
        Vec::new(),
        vec![
            RawNode::Element(c_nv_pr),
            RawNode::Element(c_nv_pic_pr),
            RawNode::Element(nv_pr),
        ],
    );

    // p:blipFill — the image reference, stretched over the whole shape.
    let rel_prefix = interner.intern(build::RELATIONSHIP_PREFIX);
    let embed = build::attr_prefixed(interner, rel_prefix, "embed", rel_id);
    let blip = build::leaf(interner, "a", DML_MAIN, "blip", vec![embed]);
    let fill_rect = build::leaf(interner, "a", DML_MAIN, "fillRect", Vec::new());
    let stretch = build::node(
        interner,
        "a",
        DML_MAIN,
        "stretch",
        Vec::new(),
        vec![RawNode::Element(fill_rect)],
    );
    let blip_fill = build::node(
        interner,
        "p",
        PML,
        "blipFill",
        Vec::new(),
        vec![RawNode::Element(blip), RawNode::Element(stretch)],
    );

    let sp_pr = build_sp_pr(interner, "rect", bounds);
    let mut picture = build::node(
        interner,
        "p",
        PML,
        "pic",
        Vec::new(),
        vec![
            RawNode::Element(nv_pic_pr),
            RawNode::Element(blip_fill),
            RawNode::Element(sp_pr),
        ],
    );
    if let Some(declaration) = rel_declaration {
        picture.attributes.push(declaration);
    }
    picture
}
