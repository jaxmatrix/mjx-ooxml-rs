//! [`Deck`] — the binding-shaped PowerPoint surface.
//!
//! A `Deck` is a [`Presentation`] with its Rust ergonomics traded for portability. Every method here
//! delegates to exactly one method there; what changes is the *shape* of the signature:
//!
//! | `mjx_pptx::Presentation`     | `mjx_ooxml::Deck`      | why |
//! |------------------------------|------------------------|-----|
//! | `impl Into<Surface>`         | [`Surface`](crate::Surface) | a generic parameter has no foreign representation |
//! | `impl Into<ShapePath>`       | [`ShapePath`](crate::ShapePath) | likewise |
//! | `usize`                      | `u32`                  | one width on every target, host-independent |
//! | `&PartName`                  | `&str`                 | a validated handle cannot cross the boundary and come back |
//! | `Option<&[u8]>`              | `Option<Vec<u8>>`      | a borrow of the deck cannot outlive the call in a binding |
//! | `Result<_, PptxError>`       | [`Result<_, Error>`](crate::Error) | sixty-five variants collapse to eleven codes |
//!
//! # What is not here, and why
//!
//! Sixteen of the surface's methods are deliberately absent. Each is unreachable through a foreign
//! function boundary, or reachable another way:
//!
//! - **`Presentation::shape`** returns a `ShapeCursor` holding `&'deck mut Presentation`. Neither
//!   PyO3 nor wasm-bindgen can express a struct that borrows another object for a caller-controlled
//!   lifetime.
//! - **`with_table_style`, `with_vml_drawing`, `edit_vml_drawing`, `with_vml_shape_for_ole_object`,
//!   `with_vml_shape_for_activex_control`** take a closure over an interner-bound reference — a
//!   borrow that is only valid for the duration of the call, and a callback that would re-enter the
//!   deck while it is mutably borrowed.
//! - **`presentation_part`, `slide_part`, `master_part`, `layout_part`** hand out part-graph
//!   identity for content that is already addressable by index through [`Surface`](crate::Surface).
//! - **`chart_rel_id`, `picture_image_rel_id`, `ole_object_rel_id`, `ole_snapshot_rel_id`,
//!   `activex_control_rel_id`, `activex_snapshot_rel_id`** hand out relationship ids for content
//!   whose bytes are readable directly (`chart_part_bytes`, `picture_image_bytes`, …).
//!
//! Part-addressed readers that are the **only** door to their content — the ink, VML and diagram
//! byte windows — are kept, with `&str` part names. [`Deck::presentation_mut`] is the Rust-only
//! escape hatch to everything above; bindings do not expose it.
//!
//! # Everything else is here
//!
//! Including the per-cell formatting setters, which the specification proposed dropping as reachable
//! through [`format_cells`](Deck::format_cells). They are not: `format_cells` skips cells covered by
//! a merge — deliberately, so formatting a region touches only what renders — while
//! [`set_cell_fill`](Deck::set_cell_fill) reaches a covered cell, whose own formatting reappears when
//! the region is unmerged. Dropping them would drop that.

use mjx_pptx::{Presentation, SlideSize};

use crate::error::{Error, ErrorCode};
use crate::format::{format_of, Format};

mod appearance;
mod bounds;
mod cells;
mod chart_decoration;
mod charts;
mod document;
mod effective;
mod hygiene;
mod hyperlinks;
mod legacy_content;
mod notes;
mod pictures;
mod shapes;
mod slides;
mod tables;
mod text;

/// An open PowerPoint deck.
///
/// ```no_run
/// use mjx_ooxml::{ColorSpec, Deck, FillSpec, PresetShapeType, ShapeBounds, SlideSize};
///
/// # fn main() -> Result<(), mjx_ooxml::Error> {
/// let mut deck = Deck::blank(SlideSize::widescreen())?;
/// let slide = deck.add_slide_from_layout(0)?;
/// let badge = deck.add_shape(
///     slide.into(),
///     PresetShapeType::Ellipse,
///     ShapeBounds::from_inches(8.0, 0.4, 1.2, 1.2),
/// )?;
/// deck.set_shape_fill(
///     slide.into(),
///     badge.into(),
///     &FillSpec::solid(ColorSpec::Srgb("1F3864".into())),
/// )?;
/// let bytes = deck.save()?;
/// # let _ = bytes;
/// # Ok(())
/// # }
/// ```
///
/// # Addressing
///
/// A [`Surface`](crate::Surface) says which shape-bearing part — `Surface::Slide(0)`,
/// `Surface::Layout(1)`, `Surface::Master(0)`, `Surface::Notes(0)`, `Surface::NotesMaster`. A
/// [`ShapePath`](crate::ShapePath) says which shape on it: `ShapePath::from(2)` for the third
/// top-level shape, `ShapePath::from([2, 1])` for member 1 of the group at index 2. Both convert
/// from a bare index, so `slide.into()` and `badge.into()` above are the whole ceremony.
///
/// # One deck, one thread
///
/// Almost every method takes `&mut self`, because reading a part materializes it. A binding
/// therefore holds a mutable borrow for the duration of each call and releases it before returning;
/// nothing here hands back a view into the deck, and nothing here takes a callback, so a second
/// borrow can never be live. Share a deck between threads by moving it, not by aliasing it.
#[derive(Debug)]
pub struct Deck {
    presentation: Presentation,
    format: Format,
}

impl Deck {
    /// A new deck with nothing in it: one slide master, one blank layout, a theme, and no slides.
    ///
    /// Nothing is read from disk and no template is embedded — every part is authored from this
    /// library's own element builders, which is what makes a deck buildable in a browser or from a
    /// `pip install` with no input file.
    ///
    /// # Errors
    /// [`ErrorCode::InvalidArgument`] if `size` is outside the `914400`..=`51206400` EMU range
    /// `p:sldSz` can express (1 to 56 inches on each axis).
    pub fn blank(size: SlideSize) -> Result<Self, Error> {
        Ok(Self {
            presentation: Presentation::blank(size)?,
            format: Format::Presentation,
        })
    }

    /// Opens a deck from the bytes of a `.pptx`, `.pptm`, `.potx`, `.ppsx` or their macro-enabled
    /// siblings.
    ///
    /// The format is [detected](crate::detect_format) from the package before anything is parsed as
    /// PresentationML, so a Word or Excel document is refused by name rather than by a parse
    /// failure, and the package is read exactly once.
    ///
    /// # Errors
    /// - [`ErrorCode::Io`] if the bytes are not a readable ZIP container.
    /// - [`ErrorCode::UnsupportedFormat`] if the package is a Word or Excel document, or an OPC
    ///   package that is not an Office document at all.
    /// - [`ErrorCode::MalformedDocument`] if it is a presentation whose `presentation.xml` or
    ///   relationships are not what the schema requires.
    pub fn open(bytes: &[u8]) -> Result<Self, Error> {
        let package = mjx_pptx::Package::open(bytes)?;
        let format = format_of(&package)?;
        if !format.is_editable() {
            return Err(Error::new(
                ErrorCode::UnsupportedFormat,
                format!(
                    "this build opens PresentationML only; these bytes are {:?} (.{}), which is not editable yet",
                    format,
                    format.conventional_extension()
                ),
            ));
        }
        Ok(Self {
            presentation: Presentation::from_package(package)?,
            format,
        })
    }

    /// Which PresentationML format this deck was opened as — a presentation, a template, a slide
    /// show, macro-enabled or not.
    ///
    /// It survives editing and saving: this library never rewrites the main part's content type, so
    /// a `.potx` opened, edited and saved is still a `.potx`. A deck built with
    /// [`blank`](Self::blank) is a [`Format::Presentation`].
    #[must_use]
    pub fn format(&self) -> Format {
        self.format
    }

    /// Serializes the deck back to `.pptx` container bytes, **after** checking its invariants.
    ///
    /// # The check is not optional
    ///
    /// [`validate`](Self::validate) runs first, and a deck that violates a packaging or a
    /// PresentationML invariant — a `p:sldId` naming a relationship that is not there, a slide the
    /// deck relates to but never lists, two shapes sharing a non-visual id — is **not written**.
    /// Those are the faults that make PowerPoint offer to repair a file. This facade inherits that
    /// guarantee rather than routing around it: a facade that lets a caller write a file the layer
    /// below refuses would be a regression, not a convenience.
    ///
    /// Every part that was not edited is re-emitted byte-identically; only parts this library
    /// actually changed are re-serialized.
    ///
    /// # Errors
    /// [`ErrorCode::InvalidDocument`] if an invariant is broken, or [`ErrorCode::Io`] if the ZIP
    /// writer fails.
    pub fn save(&self) -> Result<Vec<u8>, Error> {
        Ok(self.presentation.save()?)
    }

    /// Serializes the deck **without** checking its invariants — the deliberate override for
    /// [`save`](Self::save).
    ///
    /// Reach for it only when writing an inconsistent deck is the point: re-saving a file that was
    /// already broken when it was opened, or a state you mean to finish later. Anything this writes
    /// that [`validate`](Self::validate) would have rejected is a file PowerPoint may offer to
    /// repair.
    ///
    /// # Errors
    /// [`ErrorCode::Io`] if the ZIP writer fails.
    pub fn save_unchecked(&self) -> Result<Vec<u8>, Error> {
        Ok(self.presentation.save_unchecked()?)
    }

    /// Checks every invariant [`save`](Self::save) enforces, without writing anything.
    ///
    /// Both passes are read-only and scoped to the markup this library will actually write, so a
    /// deck opened and left alone is never faulted for markup it arrived with.
    ///
    /// # Errors
    /// [`ErrorCode::InvalidDocument`], carrying the first invariant broken as its
    /// [`source`](std::error::Error::source).
    pub fn validate(&self) -> Result<(), Error> {
        Ok(self.presentation.validate()?)
    }

    /// The underlying [`Presentation`], for reading.
    ///
    /// The Rust-only door to the sixteen methods this facade does not restate — `ShapeCursor`, the
    /// closure-taking table-style and VML readers, the part-graph accessors. Bindings do not expose
    /// it, because none of those signatures crosses a foreign function boundary.
    #[must_use]
    pub fn presentation(&self) -> &Presentation {
        &self.presentation
    }

    /// The underlying [`Presentation`], for editing. See [`presentation`](Self::presentation).
    ///
    /// This widens nothing: every invariant is enforced on [`save`](Self::save), which is the same
    /// `Presentation::save` either way. It is the *package* that stays sealed — there is no
    /// `Deck::package`, because handing out `&mut Package` would give a caller the whole part graph
    /// and make those invariants unenforceable.
    pub fn presentation_mut(&mut self) -> &mut Presentation {
        &mut self.presentation
    }

    /// Consumes the deck and returns the [`Presentation`] inside it.
    #[must_use]
    pub fn into_presentation(self) -> Presentation {
        self.presentation
    }
}

impl From<Presentation> for Deck {
    /// Wraps a presentation this crate did not open — the inverse of
    /// [`into_presentation`](Deck::into_presentation). Its [`format`](Deck::format) reports
    /// [`Format::Presentation`], since a `Presentation` carries no record of the content type it was
    /// opened under.
    fn from(presentation: Presentation) -> Self {
        Self {
            presentation,
            format: Format::Presentation,
        }
    }
}
