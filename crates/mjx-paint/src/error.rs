//! What can go wrong between a display list and a pixel.
//!
//! # Why every one of these is an error and not a panic
//!
//! `CLAUDE.md` forbids `unwrap`/`expect`/`panic!` on untrusted input, and a painter's inputs are
//! untrusted twice over: the display list may have come off a disk or a wire, and *the graphics
//! stack is not under this program's control either*. A lost device, a surface that outlived its
//! window, an adapter that vanished when a laptop switched GPUs and a driver that refuses a texture
//! size are all **normal runtime events**. An application that meets one should redraw, or fall back
//! to another painter, or tell the user — never abort.
//!
//! That is also why [`PaintError::Device`] and its neighbours carry a `String` rather than wrapping
//! a `wgpu` type: the error type of the platform boundary must not force every caller above it to
//! name `wgpu`, and R09's `tiny-skia`, PDF and SVG painters raise the same failures with no `wgpu`
//! in sight.

use mjx_scene::SceneError;

/// Anything that stops a frame reaching the screen.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PaintError {
    /// The display list could not be read.
    #[error("the display list could not be read: {0}")]
    Scene(#[from] SceneError),

    /// No graphics adapter matched what was asked for.
    ///
    /// The message names what was asked for, because "no adapter" on a machine that has three is a
    /// different problem from "no adapter" on a machine that has none.
    #[error("no graphics adapter is available ({looked_for}): {detail}")]
    NoAdapter {
        /// What was requested — the backend set and the power preference.
        looked_for: String,
        /// What the graphics stack said about it.
        detail: String,
    },

    /// An adapter was found but would not hand over a device.
    #[error("the graphics device could not be created: {0}")]
    Device(String),

    /// The driver reported an error while a frame was being built.
    ///
    /// Collected rather than panicked: `wgpu`'s default uncaptured-error handler aborts the
    /// process, which turns a shader mismatch into a crashed application. The painter installs its
    /// own handler and raises this instead.
    #[error("the graphics device reported an error: {0}")]
    DeviceReported(String),

    /// A surface could not be created for the host's window.
    #[error("no drawing surface could be created for this window: {0}")]
    Surface(String),

    /// The surface's texture could not be acquired — the window was resized, minimised or destroyed
    /// between one frame and the next.
    #[error("the drawing surface is not currently usable: {0}")]
    SurfaceLost(String),

    /// [`crate::Painter::begin`] was called while a frame was already open.
    #[error("frame {open} is still open; a painter draws one frame at a time")]
    FrameAlreadyOpen {
        /// The frame that is open.
        open: u64,
    },

    /// A [`crate::Frame`] was handed to a painter that is not drawing it.
    ///
    /// A token rather than a panic, because a frame token outliving its painter is an ordinary
    /// application bug and the painter is the layer best placed to name it.
    #[error("frame {given} is not the frame this painter is drawing ({open:?})")]
    WrongFrame {
        /// The token that was handed over.
        given: u64,
        /// The token the painter is actually drawing, if any.
        open: Option<u64>,
    },

    /// The viewport has no area.
    #[error("a {width}x{height} viewport has no pixels to draw on")]
    EmptyViewport {
        /// How wide it claimed to be.
        width: u32,
        /// How tall.
        height: u32,
    },

    /// A resource index in the list addresses a table row that is not there.
    ///
    /// The display list's own decoder answers `None` for an out-of-range index rather than
    /// panicking; this is what a painter says when it meets one, because a page drawn with a
    /// missing paint is a page drawn wrongly and silence would hide it.
    #[error("the display list's {table} table has no row {index}, which command {command} names")]
    MissingResource {
        /// Which table.
        table: &'static str,
        /// Which row was asked for.
        index: u32,
        /// Which command asked for it.
        command: usize,
    },

    /// The command stream popped more than it pushed, or ended with something still pushed.
    ///
    /// `mjx-scene`'s decoder rejects an unbalanced stream, so this is reachable only when a painter
    /// is driven by hand; it is an error rather than an assertion for the reason all of these are.
    #[error("the command stream is unbalanced at command {command}: {detail}")]
    UnbalancedStack {
        /// Where the imbalance was found.
        command: usize,
        /// What was wrong.
        detail: &'static str,
    },

    /// A texture the frame needs could not be created.
    #[error("a {width}x{height} render target could not be created: {detail}")]
    TextureUnavailable {
        /// How wide.
        width: u32,
        /// How tall.
        height: u32,
        /// Why not.
        detail: String,
    },

    /// A pooled texture handle was used after the pool had reissued its slot.
    #[error("texture handle {slot}#{generation} is stale; the pool has reissued that slot")]
    StaleTexture {
        /// Which slot.
        slot: u32,
        /// Which generation the handle claimed.
        generation: u32,
    },

    /// Reading a rendered frame back into memory failed.
    #[error("the frame could not be read back: {0}")]
    Readback(String),

    /// Two painters were asked to be compared and they are the same painter.
    ///
    /// **This is the refusal that stops the whole cross-painter exercise being vacuous.** If the
    /// `wgpu` painter is unavailable — no adapter, no driver, a machine with no graphics stack —
    /// then *"the painters agree"* degrades into *"`tiny-skia` agrees with itself"*, which is true
    /// of any painter whatever, including one that draws nothing. Refusing it in the library rather
    /// than only in a test means no caller can reach the vacuous comparison by accident.
    #[error(
        "both sides of a cross-painter comparison are `{name}`; two independent implementations \
         agreeing is evidence, and one implementation agreeing with itself is not"
    )]
    PaintersNotDistinct {
        /// What both of them call themselves.
        name: &'static str,
    },

    /// Two renders that should be comparable are not the same size.
    #[error("a {left_width}x{left_height} render cannot be compared with a {right_width}x{right_height} one")]
    IncomparableRenders {
        /// How wide the first is.
        left_width: u32,
        /// How tall.
        left_height: u32,
        /// How wide the second is.
        right_width: u32,
        /// How tall.
        right_height: u32,
    },

    /// A painter that drew a frame could not hand its pixels back.
    ///
    /// Distinct from [`PaintError::Readback`], which is a failure *during* a readback: this is a
    /// painter that answered `None` — one that drew to a window, or to a file. A comparison cannot
    /// use it and says so rather than treating an absent image as an equal one.
    #[error("painter `{name}` drew a frame and has no pixels to hand back")]
    NoPixels {
        /// Which painter.
        name: &'static str,
    },

    /// A document could not be written.
    ///
    /// The exporters' failure: a page that names a font nothing can supply, a stream that could not
    /// be built. Never a panic, for the same reason nothing else here is.
    #[error("the {format} document could not be written: {detail}")]
    Export {
        /// Which exporter.
        format: &'static str,
        /// What went wrong.
        detail: String,
    },
}
