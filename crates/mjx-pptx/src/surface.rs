//! [`Surface`] — which shape-bearing part an index-addressed call is about.
//!
//! A slide, a slide layout, and a slide master all carry the same `p:cSld > p:spTree`, so the whole
//! shape surface (text, geometry, fill, outline, effects, pictures) applies to each of them equally.
//! `Surface` is how a caller says *which*: `deck.shape_fill(0, 2)` addresses a slide, and
//! `deck.shape_fill(Surface::Layout(1), 0)` the same way addresses a layout — a plain `usize` means
//! [`Surface::Slide`], so the common case reads as if this type were not there.
//!
//! Editing a layout or master is how a change reaches *many* slides at once: a slide placeholder with
//! no explicit property of its own inherits from the same-slot placeholder on its layout, then its
//! master (see `Presentation::effective_shape_fill`).

use std::fmt;

/// The shape-bearing part an index-addressed call refers to: a slide, a slide layout, or a slide
/// master, each addressed by its own index.
///
/// `usize` converts to [`Slide`](Surface::Slide), so `0` and `Surface::Slide(0)` are interchangeable
/// wherever a surface is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Surface {
    /// A slide, indexed as in `Presentation::slide_count`.
    Slide(usize),
    /// A slide layout, indexed as in `Presentation::layout_count` (flat across masters).
    Layout(usize),
    /// A slide master, indexed as in `Presentation::master_count`.
    Master(usize),
}

impl Surface {
    /// The index within this surface's own kind.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Slide(idx) | Self::Layout(idx) | Self::Master(idx) => idx,
        }
    }

    /// The kind's name, as it appears in error messages (`slide`, `layout`, `master`).
    #[must_use]
    pub fn kind_name(self) -> &'static str {
        match self {
            Self::Slide(_) => "slide",
            Self::Layout(_) => "layout",
            Self::Master(_) => "master",
        }
    }
}

impl From<usize> for Surface {
    /// A bare index means a slide — the common case.
    fn from(index: usize) -> Self {
        Self::Slide(index)
    }
}

impl fmt::Display for Surface {
    /// `slide 0`, `layout 1`, `master 0`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.kind_name(), self.index())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_index_is_a_slide() {
        assert_eq!(Surface::from(3), Surface::Slide(3));
        let surface: Surface = 0.into();
        assert_eq!(surface, Surface::Slide(0));
    }

    #[test]
    fn index_and_display_name_the_addressed_part() {
        assert_eq!(Surface::Layout(1).index(), 1);
        assert_eq!(Surface::Master(0).index(), 0);
        assert_eq!(Surface::Slide(2).to_string(), "slide 2");
        assert_eq!(Surface::Layout(1).to_string(), "layout 1");
        assert_eq!(Surface::Master(0).to_string(), "master 0");
    }
}
