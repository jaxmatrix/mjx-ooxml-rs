//! Where a painter draws: the [`SurfaceHost`] contract, and the two hosts that ship here.
//!
//! # Why the shell owns the window and the painter does not
//!
//! Nothing in this workspace opens a window. A window belongs to the shell — `winit` on the
//! desktop, an `Activity`'s `SurfaceView` on Android, a `UIView` on iOS, a `<canvas>` in a browser —
//! and every one of those has its own event loop, its own lifecycle and its own idea of when a
//! surface stops existing. A painter that created windows would have to take a windowing library as
//! a dependency and would be wrong on at least three of those five platforms.
//!
//! So a host is an **interface the shell implements**, and it says four things: what to draw on,
//! how big it is, how many device pixels there are to a logical one, and how to ask for another
//! frame. R11's mobile surface is a fifth implementation of the same four methods, and it is what
//! forces this to be a trait rather than a struct.
//!
//! # Logical pixels and physical pixels are not the same number
//!
//! [`SurfaceHost::size`] answers in **logical** pixels — the units a window manager and a layout
//! think in — and [`SurfaceHost::scale_factor`] is how many device pixels there are to one of them.
//! A 800x600 window on a 2x display is a **1600x1200 framebuffer**, and a painter that allocated
//! 800x600 would draw a quarter of the page and stretch it.
//!
//! That is the whole reason `scale_factor` exists on this trait, and it is not decorative: it is
//! read in exactly one place — [`Viewport::physical_width`] and its partner — and every render
//! target, every scissor rectangle and every readback in this crate is sized from those two.
//! `tests/the_identity_values_are_not_the_only_values.rs` renders the same list at `1.0` and at
//! `2.0` and asserts both the size and the pixel an edge lands on, because a scale factor that is
//! only ever `1.0` is a scale factor nothing would notice the absence of.

use core::fmt;

use crate::error::PaintError;

/// What a painter is being asked to draw on.
///
/// A `wgpu` window surface is created from the platform's own window and display handles, and there
/// is no portable Rust type for those other than `raw-window-handle`'s, which `wgpu` re-exports.
/// A painter that does not use a GPU — R09's `tiny-skia`, PDF and SVG painters — ignores the
/// [`SurfaceTarget::Window`] arm and draws into memory either way.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum SurfaceTarget {
    /// A window the shell owns, described by the platform's handles.
    Window(WindowHandles),
    /// No window at all: the painter allocates its own target and the caller reads it back with
    /// [`crate::Painter::read_pixels`].
    Offscreen,
}

impl fmt::Debug for SurfaceTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // The handles are raw pointers on most platforms. Printing them would put an address in
            // a log for no benefit, and would differ between runs of the same test.
            Self::Window(_) => formatter.write_str("SurfaceTarget::Window(..)"),
            Self::Offscreen => formatter.write_str("SurfaceTarget::Offscreen"),
        }
    }
}

/// The platform handles a drawing surface is created from.
///
/// # Safety, and who carries it
///
/// These handles are borrowed, not owned: the window they describe must outlive every surface made
/// from them. That is the window system's invariant and the shell's to keep — it is the shell that
/// destroys the window — which is why this type is `Copy` and carries no lifetime it could not
/// honestly enforce. The single `unsafe` block in this crate is the one that hands them to `wgpu`,
/// and it is written where the obligation is documented.
#[derive(Clone, Copy)]
pub struct WindowHandles {
    /// The window itself.
    pub window: wgpu::rwh::RawWindowHandle,
    /// The display or connection the window belongs to, where the platform has one. X11 and Wayland
    /// do; Win32 and Cocoa do not.
    pub display: Option<wgpu::rwh::RawDisplayHandle>,
}

impl fmt::Debug for WindowHandles {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowHandles")
            .field("window", &"..")
            .field("display", &self.display.is_some())
            .finish()
    }
}

/// The surface a painter draws on, as the shell presents it.
pub trait SurfaceHost {
    /// What to draw on.
    fn raw_handle(&self) -> SurfaceTarget;

    /// How big the drawable area is, in **logical** pixels.
    fn size(&self) -> (u32, u32);

    /// How many device pixels there are to one logical pixel.
    ///
    /// `1.0` on an ordinary display, `2.0` on a Retina one, and fractional values on the Windows
    /// and GNOME scaling settings that allow them.
    fn scale_factor(&self) -> f32;

    /// Ask the shell to deliver another frame.
    ///
    /// Called by the painter when it could not finish this one — a lost surface, a device that had
    /// to be recreated — so that the application redraws rather than showing a stale window. The
    /// return value says whether the host actually scheduled anything: a host with no event loop
    /// answers `false`, and a painter that ignored the difference would report success for a frame
    /// that will never arrive.
    fn request_redraw(&self) -> bool;
}

/// The rectangle of a surface a frame is drawn into, and the scale it is drawn at.
///
/// In **logical** pixels, like [`SurfaceHost::size`]. Region composition — several viewports into
/// one surface — is loop 2's work; what this type buys now is that the painter never assumes the
/// frame covers the whole window, so that when it does not, nothing here has to change.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Viewport {
    /// Logical pixels from the surface's left edge.
    pub x: f32,
    /// Logical pixels from its top edge.
    pub y: f32,
    /// How wide, in logical pixels.
    pub width: f32,
    /// How tall.
    pub height: f32,
    /// How many device pixels to one logical pixel.
    pub scale_factor: f32,
}

impl Viewport {
    /// The whole of `host`, at the host's own scale.
    #[must_use]
    pub fn covering(host: &dyn SurfaceHost) -> Self {
        let (width, height) = host.size();
        Self {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
            scale_factor: host.scale_factor(),
        }
    }

    /// How many device pixels wide the frame's target has to be.
    ///
    /// Rounded rather than truncated: a 100.5-logical-pixel viewport at 1.0 is 101 pixels of
    /// framebuffer, and truncating would drop the last column of every fractional layout.
    #[must_use]
    pub fn physical_width(&self) -> u32 {
        physical(self.width, self.scale_factor)
    }

    /// How many device pixels tall.
    #[must_use]
    pub fn physical_height(&self) -> u32 {
        physical(self.height, self.scale_factor)
    }

    /// Device pixels from the surface's left edge.
    #[must_use]
    pub fn physical_x(&self) -> u32 {
        physical(self.x, self.scale_factor)
    }

    /// Device pixels from its top edge.
    #[must_use]
    pub fn physical_y(&self) -> u32 {
        physical(self.y, self.scale_factor)
    }

    /// Fail if this viewport has no pixels in it.
    ///
    /// # Errors
    ///
    /// [`PaintError::EmptyViewport`] when either physical dimension rounds to zero, which a
    /// minimised window and a zero-height flex row both produce.
    pub fn require_pixels(&self) -> Result<(u32, u32), PaintError> {
        let (width, height) = (self.physical_width(), self.physical_height());
        if width == 0 || height == 0 {
            return Err(PaintError::EmptyViewport { width, height });
        }
        Ok((width, height))
    }
}

/// A logical length in device pixels, clamped to something a texture allocator can act on.
fn physical(logical: f32, scale_factor: f32) -> u32 {
    let scale = if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        // A non-finite or non-positive scale is a gesture divided by zero, not an instruction to
        // allocate nothing: `mjx_text::DeviceScale` takes the same view of the same accident.
        1.0
    };
    let pixels = (logical * scale).round();
    if !pixels.is_finite() || pixels <= 0.0 {
        return 0;
    }
    // A surface larger than this is larger than any adapter's `max_texture_dimension_2d`, so
    // clamping here turns "allocate 4 billion pixels" into a target the device will merely refuse.
    pixels.min(u32::MAX as f32) as u32
}

/// A target with no window: the painter allocates a texture and the caller reads it back.
///
/// What every test in this crate draws on, what R10's golden images are taken from, and what a
/// thumbnail or a print preview uses in the real application.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct OffscreenSurface {
    width: u32,
    height: u32,
    scale_factor: f32,
}

impl OffscreenSurface {
    /// A `width` x `height` logical-pixel target at `scale_factor` device pixels to the logical one.
    #[must_use]
    pub fn new(width: u32, height: u32, scale_factor: f32) -> Self {
        Self {
            width,
            height,
            scale_factor,
        }
    }
}

impl SurfaceHost for OffscreenSurface {
    fn raw_handle(&self) -> SurfaceTarget {
        SurfaceTarget::Offscreen
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    fn request_redraw(&self) -> bool {
        // There is no event loop behind an offscreen target, and saying otherwise would let a
        // painter report "I asked for another frame" when nothing will ever deliver one.
        false
    }
}

/// A window the shell owns, described by its handles.
///
/// The shell constructs one of these from its own window each time the window is resized or
/// rescaled, and hands it to the painter. It deliberately does **not** hold the window: a painter
/// that owned a `winit::Window` would make this crate depend on `winit`, and there are five
/// windowing systems in this platform's matrix.
pub struct DesktopWindow {
    handles: WindowHandles,
    width: u32,
    height: u32,
    scale_factor: f32,
    redraw: Option<Box<dyn Fn() + Send + Sync>>,
}

impl DesktopWindow {
    /// A window of `width` x `height` logical pixels at `scale_factor`, drawn on through `handles`.
    ///
    /// # Safety obligation, carried by the caller
    ///
    /// `handles` must describe a live window, and that window must outlive every frame the painter
    /// draws through it. This constructor is safe because breaking that promise is the shell's
    /// doing and not this crate's; the `unsafe` block that acts on the promise is in
    /// [`crate::backend`], written beside the justification.
    #[must_use]
    pub fn new(handles: WindowHandles, width: u32, height: u32, scale_factor: f32) -> Self {
        Self {
            handles,
            width,
            height,
            scale_factor,
            redraw: None,
        }
    }

    /// The same window, with a way to ask the shell's event loop for another frame.
    #[must_use]
    pub fn requesting_redraws_through(mut self, redraw: Box<dyn Fn() + Send + Sync>) -> Self {
        self.redraw = Some(redraw);
        self
    }
}

impl fmt::Debug for DesktopWindow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DesktopWindow")
            .field("handles", &self.handles)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("scale_factor", &self.scale_factor)
            .field("can_request_redraws", &self.redraw.is_some())
            .finish()
    }
}

impl SurfaceHost for DesktopWindow {
    fn raw_handle(&self) -> SurfaceTarget {
        SurfaceTarget::Window(self.handles)
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    fn request_redraw(&self) -> bool {
        match &self.redraw {
            Some(redraw) => {
                redraw();
                true
            }
            None => false,
        }
    }
}
