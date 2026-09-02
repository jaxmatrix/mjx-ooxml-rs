//! Pictures and media: adding an image, reading and replacing the bytes behind one, and
//! neutralizing references that point outside the package.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::count;
use crate::{Deck, Error, LinkedImage, MediaReference, ShapeBounds, ShapePath, Surface};

impl Deck {
    /// Appends a picture (`p:pic`) showing `bytes` to `surface`, laid out at `bounds`. Returns the
    /// index of the new shape in the slide's one shape index space (see `shape_count`); `shape_kind`
    /// reports it as `ShapeKind::Picture`, and the whole `p:spPr` surface — outline, effects, geometry
    /// — applies to it like any other shape.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_picture`](mjx_pptx::Presentation::add_picture).
    pub fn add_picture(
        &mut self,
        surface: Surface,
        bytes: &[u8],
        bounds: ShapeBounds,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.add_picture(
            surface.to_model(),
            bytes,
            bounds,
        )?))
    }

    /// Every audio/video/media relationship on `surface`, with where each is referenced from and
    /// whether it is external.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::media_references`](mjx_pptx::Presentation::media_references).
    pub fn media_references(&mut self, surface: Surface) -> Result<Vec<MediaReference>, Error> {
        Ok(self.presentation.media_references(surface.to_model())?)
    }

    /// Replaces the media that relationship `rel_id` on `surface` binds with an in-package placeholder,
    /// so a reference to unreachable external audio/video resolves inside the package instead. The
    /// placeholder is `placeholder` if given, else a built-in one matching the media kind — a valid
    /// silent WAV for audio (`default_placeholder_audio`) or a minimal MP4 for video
    /// (`default_placeholder_video`). The relationship is retargeted at the placeholder, so every
    /// carrier that named it — the `p:pic`, its `a14:media` fallback, timing/transition sounds — now
    /// resolves locally; the poster image is untouched.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::replace_media_with_placeholder`](mjx_pptx::Presentation::replace_media_with_placeholder).
    pub fn replace_media_with_placeholder(
        &mut self,
        surface: Surface,
        rel_id: &str,
        placeholder: Option<&[u8]>,
    ) -> Result<(), Error> {
        Ok(self.presentation.replace_media_with_placeholder(
            surface.to_model(),
            rel_id,
            placeholder,
        )?)
    }

    /// The target of the image that picture `shape_idx` on `surface` *links* (`p:blipFill >
    /// a:blip@r:link`), exactly as the relationship records it — an external path/URL for the common
    /// case, or an in-package part target for an internal link. `None` when the picture embeds its
    /// image (or binds none): an embedded image has no separate target, its bytes are the image.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::picture_image_link_target`](mjx_pptx::Presentation::picture_image_link_target).
    pub fn picture_image_link_target(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<String>, Error> {
        Ok(self
            .presentation
            .picture_image_link_target(surface.to_model(), shape_idx.to_model())?)
    }

    /// The stored bytes of the image that picture `shape_idx` on `surface` binds, exactly as the
    /// package holds them (never decoded or re-encoded), or `None` when the picture binds no image.
    /// Borrowed from the package, so a large image is not copied.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::picture_image_bytes`](mjx_pptx::Presentation::picture_image_bytes).
    pub fn picture_image_bytes(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .presentation
            .picture_image_bytes(surface.to_model(), shape_idx.to_model())?
            .map(<[u8]>::to_vec))
    }

    /// Points picture `shape_idx` on `surface` at `bytes`, adding the image to the package if it is not
    /// already there (`add_image`, so identical bytes are stored once) and rewriting the blip's
    /// `@r:embed`. Any `@r:link` is dropped — the picture now embeds its image — and the rest of the
    /// `p:blipFill` (source rect, tile/stretch) is preserved.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_picture_image`](mjx_pptx::Presentation::set_picture_image).
    pub fn set_picture_image(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        bytes: &[u8],
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_picture_image(surface.to_model(), shape_idx.to_model(), bytes)?)
    }

    /// Every picture on `surface` that *links* its image (`a:blip@r:link`) rather than embedding it,
    /// with where each links from — the candidates for `replace_linked_image_with_placeholder`. A
    /// linked image is the common source that can be unreachable on another platform; this saves the
    /// caller from walking the shapes themselves. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::linked_images`](mjx_pptx::Presentation::linked_images).
    pub fn linked_images(&mut self, surface: Surface) -> Result<Vec<LinkedImage>, Error> {
        Ok(self.presentation.linked_images(surface.to_model())?)
    }

    /// Replaces the *linked* image of picture `shape_idx` on `surface` with an embedded placeholder, so
    /// a picture that points at an unreachable external file resolves inside the package instead. The
    /// placeholder is `placeholder` if given, else `DEFAULT_PLACEHOLDER_IMAGE`. The picture becomes an
    /// ordinary embedded picture (`@r:link` → `@r:embed`), keeping its bounds and the rest of its
    /// `p:blipFill`, and the now-unused link relationship is dropped.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::replace_linked_image_with_placeholder`](mjx_pptx::Presentation::replace_linked_image_with_placeholder).
    pub fn replace_linked_image_with_placeholder(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        placeholder: Option<&[u8]>,
    ) -> Result<(), Error> {
        Ok(self.presentation.replace_linked_image_with_placeholder(
            surface.to_model(),
            shape_idx.to_model(),
            placeholder,
        )?)
    }

    /// Stores `bytes` as an image part of the package and relates it to `surface`, returning the
    /// **slide-scoped relationship id** that names the image — the `rel_id` to hand to
    /// `FillSpec::Picture` via `set_shape_fill`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_image`](mjx_pptx::Presentation::add_image).
    pub fn add_image(&mut self, surface: Surface, bytes: &[u8]) -> Result<String, Error> {
        Ok(self.presentation.add_image(surface.to_model(), bytes)?)
    }
}
