//! Legacy and preserved content: OLE objects, ActiveX controls, ink, SmartArt diagrams, and the VML
//! drawings that back the first two.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::{count, index, part_name};
use crate::{
    ActiveXControlSpec, ActiveXPersistence, Deck, DiagramContent, DiagramPartKind, DiagramParts,
    DiagramRelationshipIds, Error, InkReference, OleObject, OleObjectSpec, ShapeBounds, ShapePath,
    Surface,
};

impl Deck {
    /// The raw bytes of the embedded object the OLE frame `shape_idx` on `surface` references
    /// (`/ppt/embeddings/oleObjectN.bin` or an embedded package), exactly as the package holds them, or
    /// `None` when the shape frames no OLE object. Borrowed from the package, so the part is not
    /// copied.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::ole_object_part_bytes`](mjx_pptx::Presentation::ole_object_part_bytes).
    pub fn ole_object_part_bytes(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .presentation
            .ole_object_part_bytes(surface.to_model(), shape_idx.to_model())?
            .map(<[u8]>::to_vec))
    }

    /// The stored bytes of the OLE fallback snapshot image the frame `shape_idx` on `surface` embeds,
    /// exactly as the package holds them (never decoded or re-encoded), or `None` when the frame is not
    /// an OLE object or carries no snapshot. Borrowed from the package.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::ole_snapshot_image_bytes`](mjx_pptx::Presentation::ole_snapshot_image_bytes).
    pub fn ole_snapshot_image_bytes(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .presentation
            .ole_snapshot_image_bytes(surface.to_model(), shape_idx.to_model())?
            .map(<[u8]>::to_vec))
    }

    /// The `progId` the OLE frame `shape_idx` on `surface` declares (e.g. `"Excel.Sheet.12"`) — the
    /// application that owns the embedded object — or `None` when the shape frames no OLE object or the
    /// attribute is absent. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::ole_prog_id`](mjx_pptx::Presentation::ole_prog_id).
    pub fn ole_prog_id(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<String>, Error> {
        Ok(self
            .presentation
            .ole_prog_id(surface.to_model(), shape_idx.to_model())?)
    }

    /// Every OLE object frame on `surface`, with where its object data is referenced from and whether
    /// that reference is external.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::ole_objects`](mjx_pptx::Presentation::ole_objects).
    pub fn ole_objects(&mut self, surface: Surface) -> Result<Vec<OleObject>, Error> {
        Ok(self.presentation.ole_objects(surface.to_model())?)
    }

    /// Replaces the object data of the OLE frame `shape_idx` on `surface` with an in-package
    /// placeholder, so an object that points at unreachable external data resolves inside the package
    /// instead. The placeholder is `placeholder` if given, else `default_placeholder_ole` (a minimal
    /// valid compound file). The `p:oleObj` markup is unchanged — its relationship is simply retargeted
    /// at the placeholder — and the object keeps displaying via its snapshot image.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::replace_ole_object_with_placeholder`](mjx_pptx::Presentation::replace_ole_object_with_placeholder).
    pub fn replace_ole_object_with_placeholder(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        placeholder: Option<&[u8]>,
    ) -> Result<(), Error> {
        Ok(self.presentation.replace_ole_object_with_placeholder(
            surface.to_model(),
            shape_idx.to_model(),
            placeholder,
        )?)
    }

    /// The number of legacy **ActiveX** form controls on `surface` (`p:cSld > p:controls > p:control`).
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::activex_control_count`](mjx_pptx::Presentation::activex_control_count).
    pub fn activex_control_count(&mut self, surface: Surface) -> Result<u32, Error> {
        Ok(count(
            self.presentation
                .activex_control_count(surface.to_model())?,
        ))
    }

    /// The `name` the ActiveX control `control_idx` on `surface` declares (e.g. `"CommandButton1"`), or
    /// `None` when there is no such control or it is unnamed. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::activex_control_name`](mjx_pptx::Presentation::activex_control_name).
    pub fn activex_control_name(
        &mut self,
        surface: Surface,
        control_idx: u32,
    ) -> Result<Option<String>, Error> {
        Ok(self
            .presentation
            .activex_control_name(surface.to_model(), index(control_idx))?)
    }

    /// The raw bytes of the ActiveX control part (`ax:ocx` markup) the control `control_idx` on
    /// `surface` references, exactly as the package holds them, or `None` when there is no such
    /// control. Borrowed from the package; reading does not dirty anything.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::activex_part_bytes`](mjx_pptx::Presentation::activex_part_bytes).
    pub fn activex_part_bytes(
        &mut self,
        surface: Surface,
        control_idx: u32,
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .presentation
            .activex_part_bytes(surface.to_model(), index(control_idx))?
            .map(<[u8]>::to_vec))
    }

    /// The ActiveX control's **persisted state** — the bytes of `/ppt/activeX/activeXN.bin` — for the
    /// control `control_idx` on `surface`, or `None` when there is no such control or it persists no
    /// state. Borrowed from the package; reading does not dirty anything.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::activex_state_bytes`](mjx_pptx::Presentation::activex_state_bytes).
    pub fn activex_state_bytes(
        &mut self,
        surface: Surface,
        control_idx: u32,
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .presentation
            .activex_state_bytes(surface.to_model(), index(control_idx))?
            .map(<[u8]>::to_vec))
    }

    /// The stored bytes of the ActiveX control's fallback snapshot image for the control `control_idx`
    /// on `surface`, exactly as the package holds them (never decoded or re-encoded), or `None` when
    /// there is no such control or snapshot. Borrowed from the package.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::activex_snapshot_image_bytes`](mjx_pptx::Presentation::activex_snapshot_image_bytes).
    pub fn activex_snapshot_image_bytes(
        &mut self,
        surface: Surface,
        control_idx: u32,
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .presentation
            .activex_snapshot_image_bytes(surface.to_model(), index(control_idx))?
            .map(<[u8]>::to_vec))
    }

    #[cfg(feature = "vml")]
    /// The names of every legacy **VML** drawing part in the package (`ppt/drawings/vmlDrawingN.vml`
    /// and the like), in package order.
    ///
    /// See [`Presentation::vml_part_names`](mjx_pptx::Presentation::vml_part_names).
    #[must_use]
    pub fn vml_part_names(&self) -> Vec<String> {
        self.presentation
            .vml_part_names()
            .into_iter()
            .map(|p| p.as_str().to_owned())
            .collect()
    }

    #[cfg(feature = "vml")]
    /// The raw bytes of the VML drawing `part`, exactly as the package holds them, or `None` when the
    /// package has no such part (or it has been edited elsewhere). Borrowed from the package, so the
    /// part is not copied and nothing is dirtied.
    ///
    /// See [`Presentation::vml_part_bytes`](mjx_pptx::Presentation::vml_part_bytes).
    #[must_use]
    pub fn vml_part_bytes(&self, part: &str) -> Option<Vec<u8>> {
        self.presentation
            .vml_part_bytes(&part_name(part).ok()?)
            .map(<[u8]>::to_vec)
    }

    /// The names of every **ink** (InkML) part in the package (`ppt/ink/inkN.xml`), in package order.
    ///
    /// See [`Presentation::ink_part_names`](mjx_pptx::Presentation::ink_part_names).
    #[must_use]
    pub fn ink_part_names(&self) -> Vec<String> {
        self.presentation
            .ink_part_names()
            .into_iter()
            .map(|p| p.as_str().to_owned())
            .collect()
    }

    /// The raw bytes of the ink (InkML) `part`, exactly as the package holds them, or `None` when the
    /// package has no such part (or it has been edited elsewhere). Borrowed from the package, so the
    /// part is not copied and nothing is dirtied.
    ///
    /// See [`Presentation::ink_part_bytes`](mjx_pptx::Presentation::ink_part_bytes).
    #[must_use]
    pub fn ink_part_bytes(&self, part: &str) -> Option<Vec<u8>> {
        self.presentation
            .ink_part_bytes(&part_name(part).ok()?)
            .map(<[u8]>::to_vec)
    }

    /// Every ink (InkML) part `surface` references, with where it is referenced from.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::ink_references`](mjx_pptx::Presentation::ink_references).
    pub fn ink_references(&mut self, surface: Surface) -> Result<Vec<InkReference>, Error> {
        Ok(self
            .presentation
            .ink_references(surface.to_model())?
            .into_iter()
            .map(InkReference::from)
            .collect())
    }

    /// The ink part the shape `shape_idx` on `surface` references, or `None` when that shape is not a
    /// content part or does not reference ink.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::ink_part_for_shape`](mjx_pptx::Presentation::ink_part_for_shape).
    pub fn ink_part_for_shape(
        &mut self,
        surface: Surface,
        shape_idx: u32,
    ) -> Result<Option<String>, Error> {
        Ok(self
            .presentation
            .ink_part_for_shape(surface.to_model(), index(shape_idx))?
            .map(|p| p.as_str().to_owned()))
    }

    /// The shape index of the content part on `surface` that references the ink `part`, or `None` when
    /// no shape on that surface does (or the reference lives inside an `mc:AlternateContent`, which is
    /// out of the shape index space).
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_for_ink_part`](mjx_pptx::Presentation::shape_for_ink_part).
    pub fn shape_for_ink_part(
        &mut self,
        surface: Surface,
        part: &str,
    ) -> Result<Option<u32>, Error> {
        Ok(self
            .presentation
            .shape_for_ink_part(surface.to_model(), &part_name(part)?)?
            .map(count))
    }

    /// Adds an ink (InkML) part holding `inkml` to the package and a `p:contentPart` referencing it to
    /// `surface`, and returns the new shape's index in the one shape index space.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_ink`](mjx_pptx::Presentation::add_ink).
    pub fn add_ink(&mut self, surface: Surface, inkml: &[u8]) -> Result<u32, Error> {
        Ok(count(self.presentation.add_ink(surface.to_model(), inkml)?))
    }

    /// Replaces the strokes of the ink the shape `shape_idx` on `surface` references, in place.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_ink_content`](mjx_pptx::Presentation::set_ink_content).
    pub fn set_ink_content(
        &mut self,
        surface: Surface,
        shape_idx: u32,
        inkml: &[u8],
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_ink_content(surface.to_model(), index(shape_idx), inkml)?)
    }

    /// The four relationship ids the SmartArt frame `shape_idx` on `surface` names in its `dgm:relIds`,
    /// or `None` when the shape frames no diagram. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::diagram_relationship_ids`](mjx_pptx::Presentation::diagram_relationship_ids).
    pub fn diagram_relationship_ids(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<DiagramRelationshipIds>, Error> {
        Ok(self
            .presentation
            .diagram_relationship_ids(surface.to_model(), shape_idx.to_model())?)
    }

    /// The parts of the SmartArt diagram the frame `shape_idx` on `surface` references, resolved to
    /// part names — the relationship graph behind the diagram, `None` when the shape frames none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::diagram_parts`](mjx_pptx::Presentation::diagram_parts).
    pub fn diagram_parts(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<DiagramParts>, Error> {
        Ok(self
            .presentation
            .diagram_parts(surface.to_model(), shape_idx.to_model())?
            .map(DiagramParts::from))
    }

    /// The raw bytes of a diagram `part`, exactly as the package holds them, or `None` when the package
    /// has no such part (or it has been edited elsewhere). Borrowed; nothing is dirtied.
    ///
    /// See [`Presentation::diagram_part_bytes`](mjx_pptx::Presentation::diagram_part_bytes).
    #[must_use]
    pub fn diagram_part_bytes(&self, part: &str) -> Option<Vec<u8>> {
        self.presentation
            .diagram_part_bytes(&part_name(part).ok()?)
            .map(<[u8]>::to_vec)
    }

    /// Adds a SmartArt diagram to `surface`, laid out inside `bounds`, and returns its index in the
    /// shape tree.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_diagram`](mjx_pptx::Presentation::add_diagram).
    pub fn add_diagram(
        &mut self,
        surface: Surface,
        content: &DiagramContent,
        bounds: ShapeBounds,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.add_diagram(
            surface.to_model(),
            content,
            bounds,
        )?))
    }

    /// Replaces one part of the SmartArt diagram the frame `shape_idx` on `surface` references, in
    /// place.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_diagram_part`](mjx_pptx::Presentation::set_diagram_part).
    pub fn set_diagram_part(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        kind: DiagramPartKind,
        bytes: Vec<u8>,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_diagram_part(
            surface.to_model(),
            shape_idx.to_model(),
            kind,
            bytes,
        )?)
    }

    /// Adds an OLE object to `surface`, laid out inside `bounds`, and returns its index in the shape
    /// tree.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_ole_object`](mjx_pptx::Presentation::add_ole_object).
    pub fn add_ole_object(
        &mut self,
        surface: Surface,
        spec: &OleObjectSpec<'_>,
        bounds: ShapeBounds,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.add_ole_object(
            surface.to_model(),
            spec,
            bounds,
        )?))
    }

    /// Sets the `progId` of the OLE frame `shape_idx` on `surface` — which application owns the
    /// embedded object. Only the surface's part is dirtied.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_ole_prog_id`](mjx_pptx::Presentation::set_ole_prog_id).
    pub fn set_ole_prog_id(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        prog_id: &str,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_ole_prog_id(surface.to_model(), shape_idx.to_model(), prog_id)?)
    }

    /// Replaces the data of the OLE object the frame `shape_idx` on `surface` embeds, in place.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_ole_object_data`](mjx_pptx::Presentation::set_ole_object_data).
    pub fn set_ole_object_data(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        bytes: &[u8],
    ) -> Result<(), Error> {
        Ok(self.presentation.set_ole_object_data(
            surface.to_model(),
            shape_idx.to_model(),
            bytes,
        )?)
    }

    /// Replaces the fallback snapshot image of the OLE frame `shape_idx` on `surface` — the picture a
    /// consumer draws in place of the object it will never run.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_ole_snapshot_image`](mjx_pptx::Presentation::set_ole_snapshot_image).
    pub fn set_ole_snapshot_image(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        bytes: &[u8],
    ) -> Result<(), Error> {
        Ok(self.presentation.set_ole_snapshot_image(
            surface.to_model(),
            shape_idx.to_model(),
            bytes,
        )?)
    }

    /// Adds an ActiveX form control to `surface`, laid out inside `bounds`, and returns its index in
    /// the surface's **control** index space (not the shape index space — a `p:control` is a sibling of
    /// the shape tree, not a member of it).
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_activex_control`](mjx_pptx::Presentation::add_activex_control).
    pub fn add_activex_control(
        &mut self,
        surface: Surface,
        spec: &ActiveXControlSpec<'_>,
        bounds: ShapeBounds,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.add_activex_control(
            surface.to_model(),
            spec,
            bounds,
        )?))
    }

    /// Points the OLE frame `shape_idx` on `surface` at the VML shape with `identifier`
    /// (`p:oleObj@spid`) — how an authored object is bound to the legacy fallback that draws it.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_ole_legacy_shape_id`](mjx_pptx::Presentation::set_ole_legacy_shape_id).
    pub fn set_ole_legacy_shape_id(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        identifier: &str,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_ole_legacy_shape_id(
            surface.to_model(),
            shape_idx.to_model(),
            identifier,
        )?)
    }

    /// Points the ActiveX control `control_idx` on `surface` at the VML shape with `identifier`
    /// (`p:control@spid`). As `set_ole_legacy_shape_id`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_activex_control_shape_id`](mjx_pptx::Presentation::set_activex_control_shape_id).
    pub fn set_activex_control_shape_id(
        &mut self,
        surface: Surface,
        control_idx: u32,
        identifier: &str,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_activex_control_shape_id(
            surface.to_model(),
            index(control_idx),
            identifier,
        )?)
    }

    /// The `spid` the ActiveX control `control_idx` on `surface` names — the `id` of the VML shape that
    /// draws it in a legacy consumer — or `None` when there is no such control or it names none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::activex_control_shape_id`](mjx_pptx::Presentation::activex_control_shape_id).
    pub fn activex_control_shape_id(
        &mut self,
        surface: Surface,
        control_idx: u32,
    ) -> Result<Option<String>, Error> {
        Ok(self
            .presentation
            .activex_control_shape_id(surface.to_model(), index(control_idx))?)
    }

    /// The COM class id the ActiveX control `control_idx` on `surface` names (`ax:ocx@ax:classid`), or
    /// `None` when there is no such control or its part states none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::activex_class_id`](mjx_pptx::Presentation::activex_class_id).
    pub fn activex_class_id(
        &mut self,
        surface: Surface,
        control_idx: u32,
    ) -> Result<Option<String>, Error> {
        Ok(self
            .presentation
            .activex_class_id(surface.to_model(), index(control_idx))?)
    }

    /// How the ActiveX control `control_idx` on `surface` persists its state (`ax:ocx@ax:persistence`),
    /// or `None` when there is no such control, its part states none, or it names a value the ActiveX
    /// part does not define.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::activex_persistence`](mjx_pptx::Presentation::activex_persistence).
    pub fn activex_persistence(
        &mut self,
        surface: Surface,
        control_idx: u32,
    ) -> Result<Option<ActiveXPersistence>, Error> {
        Ok(self
            .presentation
            .activex_persistence(surface.to_model(), index(control_idx))?)
    }

    /// Renames the ActiveX control `control_idx` on `surface` (`p:control@name`). Only the surface's
    /// part is dirtied.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_activex_control_name`](mjx_pptx::Presentation::set_activex_control_name).
    pub fn set_activex_control_name(
        &mut self,
        surface: Surface,
        control_idx: u32,
        name: &str,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_activex_control_name(
            surface.to_model(),
            index(control_idx),
            name,
        )?)
    }

    /// Replaces the persisted state of the ActiveX control `control_idx` on `surface`, in place.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_activex_state`](mjx_pptx::Presentation::set_activex_state).
    pub fn set_activex_state(
        &mut self,
        surface: Surface,
        control_idx: u32,
        state: &[u8],
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_activex_state(surface.to_model(), index(control_idx), state)?)
    }

    /// Replaces the fallback snapshot image of the ActiveX control `control_idx` on `surface` — the
    /// picture a consumer draws in place of the control it will never run.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_activex_snapshot_image`](mjx_pptx::Presentation::set_activex_snapshot_image).
    pub fn set_activex_snapshot_image(
        &mut self,
        surface: Surface,
        control_idx: u32,
        bytes: &[u8],
    ) -> Result<(), Error> {
        Ok(self.presentation.set_activex_snapshot_image(
            surface.to_model(),
            index(control_idx),
            bytes,
        )?)
    }

    /// Removes the ActiveX control `control_idx` from `surface`, closing the gap in the control index
    /// space. Only the surface's part is dirtied.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::remove_activex_control`](mjx_pptx::Presentation::remove_activex_control).
    pub fn remove_activex_control(
        &mut self,
        surface: Surface,
        control_idx: u32,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .remove_activex_control(surface.to_model(), index(control_idx))?)
    }

    /// The `spid` the OLE frame `shape_idx` on `surface` names — the `id` of the VML shape that draws
    /// it in a legacy consumer — or `None` when the shape frames no OLE object or names no `spid`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::ole_legacy_shape_id`](mjx_pptx::Presentation::ole_legacy_shape_id).
    pub fn ole_legacy_shape_id(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<String>, Error> {
        Ok(self
            .presentation
            .ole_legacy_shape_id(surface.to_model(), shape_idx.to_model())?)
    }

    #[cfg(feature = "vml")]
    /// The legacy VML drawing part `surface` relates to, or `None` when it has none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::vml_drawing_part`](mjx_pptx::Presentation::vml_drawing_part).
    pub fn vml_drawing_part(&self, surface: Surface) -> Result<Option<String>, Error> {
        Ok(self
            .presentation
            .vml_drawing_part(surface.to_model())?
            .map(|p| p.as_str().to_owned()))
    }

    #[cfg(feature = "vml")]
    /// Stores `drawing` as a new legacy VML drawing part and relates it to `surface`, returning the
    /// part's name.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_vml_drawing`](mjx_pptx::Presentation::add_vml_drawing).
    pub fn add_vml_drawing(&mut self, surface: Surface, drawing: &[u8]) -> Result<String, Error> {
        Ok(self
            .presentation
            .add_vml_drawing(surface.to_model(), drawing)?
            .as_str()
            .to_owned())
    }
}
