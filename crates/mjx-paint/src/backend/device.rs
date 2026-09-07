//! Getting a graphics device, and **saying honestly what was got**.
//!
//! # The whole point of this module
//!
//! `wgpu` is accommodating. Ask for Vulkan on a machine that has none and it will hand back OpenGL.
//! Ask for a discrete GPU on a laptop that switched to its integrated one and it will hand back the
//! integrated one. Ask for four-sample multisampling on a format that does not support it and the
//! pipeline will simply be built at one. Every one of those is the right behaviour for an
//! application and the wrong behaviour for a gate, because *"the frame rendered"* stays true through
//! all of them.
//!
//! So nothing here reports what was **asked for**. [`GraphicsDevice::report`] is read off the live
//! adapter and the live device after they exist, and the sample count it states is the count the
//! pipelines were actually built at. A test can then assert *which* painter and *which* backend ran,
//! which is the only form of that assertion that cannot pass on a machine where the work was
//! skipped.
//!
//! # The null backend is not compiled in, on purpose
//!
//! `wgpu` 30 ships a `noop` backend that accepts every command and draws nothing. It is genuinely
//! useful for testing `wgpu` itself, and it is exactly the thing this child was warned about: with
//! it compiled in, a machine with no graphics stack at all would still produce a painter, a device
//! and a successful frame, and every "did it render" assertion would pass against a black hole. The
//! workspace manifest states the feature list in full so that the absence is visible where the
//! decision is made, and [`GraphicsDevice::open`] asks only for backends that draw.
//!
//! # Errors are collected, never fatal
//!
//! `wgpu`'s default uncaptured-error handler panics the process. A painter may not panic, so this
//! installs its own: driver complaints are collected and surfaced as
//! [`PaintError::DeviceReported`] at the end of a frame, where a caller can act on them.

use std::sync::{Arc, Mutex};

use crate::error::PaintError;
use crate::painter::{AdapterKind, Antialiasing, BackendReport, GraphicsApi};

/// How many samples per pixel this painter asks for.
///
/// Four, and multisampling rather than any analytic scheme, because **it runs on every backend from
/// WebGL 2 to Vulkan** — that is the constraint that chose it, and it is the same constraint that
/// keeps a compute stage out of this crate. Eight samples is supported on desktop hardware and on
/// almost nothing else; two is barely better than none on a near-horizontal edge.
pub const REQUESTED_SAMPLE_COUNT: u32 = 4;

/// The colour format every offscreen target and every effect target is in.
///
/// `Rgba8Unorm` rather than `Rgba8UnormSrgb`: every colour in a display list is already sRGB
/// eight-bit, `mjx_tokens::Color` is eight-bit sRGB, and a target that converted to linear on write
/// and back on read would round-trip every value through a different set of eight bits — visible as
/// banding in a gradient, and as a shifted colour in a golden image. Blending in sRGB is what every
/// document renderer this platform is compared against does.
pub const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// The depth/stencil format the clip stack lives in.
///
/// Stencil is what makes a nested path clip exact. `Depth24PlusStencil8` rather than `Stencil8`
/// because the former is required of every WebGPU implementation and the latter is optional, and
/// the baseline backend is the one that decides.
pub const STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

/// A live graphics device, and what it actually is.
pub struct GraphicsDevice {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    info: wgpu::AdapterInfo,
    sample_count: u32,
    complaints: Arc<Mutex<Vec<String>>>,
}

impl core::fmt::Debug for GraphicsDevice {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("GraphicsDevice")
            .field("backend", &self.info.backend)
            .field("adapter", &self.info.name)
            .field("device_type", &self.info.device_type)
            .field("sample_count", &self.sample_count)
            .finish()
    }
}

impl GraphicsDevice {
    /// Open a device on whatever backend this build and this machine agree on.
    ///
    /// `surface` is the surface the adapter must be able to present to. It is not optional in
    /// spirit: **WebGL 2 refuses to choose an adapter without one**, so a browser build that asked
    /// for a device before it had a canvas would get nothing and could not say why.
    ///
    /// # Errors
    ///
    /// [`PaintError::NoAdapter`] when no adapter matches — which on a machine with no graphics stack
    /// is the normal answer and not a bug — and [`PaintError::Device`] when one is found but will
    /// not open.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(surface: Option<&wgpu::Surface<'static>>) -> Result<Self, PaintError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        Self::open_on(instance, surface, drawing_backends())
    }

    /// The same, on an instance and a backend set the caller chose.
    ///
    /// Public so a gate can prove the failure path: pointing this at
    /// [`wgpu::Backends::empty()`] is how `tests/a_skipped_gpu_is_a_loud_skip.rs` shows that a
    /// missing adapter is a named error rather than a silently green frame.
    ///
    /// # Errors
    ///
    /// As [`GraphicsDevice::open`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_on(
        instance: wgpu::Instance,
        surface: Option<&wgpu::Surface<'static>>,
        backends: wgpu::Backends,
    ) -> Result<Self, PaintError> {
        pollster::block_on(Self::open_on_async(instance, surface, backends))
    }

    /// The same, awaited rather than blocked on.
    ///
    /// **This is the only form available in a browser, and the reason both exist.** Adapter and
    /// device acquisition are futures on every target; on native they are already complete when
    /// they are returned, so `pollster` turns them back into ordinary calls and nothing in this
    /// workspace needs an async runtime. In a browser they are genuinely asynchronous — the answer
    /// arrives on the event loop — and blocking a `wasm32` thread on one **never returns**. So the
    /// blocking constructors are compiled out there rather than left as a trap, and a browser shell
    /// awaits this.
    ///
    /// # Errors
    ///
    /// As [`GraphicsDevice::open`].
    pub async fn open_on_async(
        instance: wgpu::Instance,
        surface: Option<&wgpu::Surface<'static>>,
        backends: wgpu::Backends,
    ) -> Result<Self, PaintError> {
        let request = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            // Never. A fallback adapter is a software rasteriser chosen *silently*, which is the one
            // thing this whole module exists to prevent. A machine that only has a software
            // implementation still gets it — `lavapipe` is an ordinary Vulkan adapter and is
            // enumerated as one — and `report().adapter` then says `Cpu`, which is the honest answer
            // and is what a gate reads.
            force_fallback_adapter: false,
            compatible_surface: surface,
            apply_limit_buckets: false,
        };
        let adapter = match instance.request_adapter(&request).await {
            Ok(adapter) => adapter,
            Err(error) => {
                return Err(PaintError::NoAdapter {
                    looked_for: format!("{backends:?}, high performance"),
                    detail: error.to_string(),
                })
            }
        };
        // A backend outside the requested set is not an error `wgpu` raises — it simply picks one —
        // so it is checked here, where a caller that asked for one thing and got another can be told.
        let info = adapter.get_info();
        if !backends.contains(wgpu::Backends::from(info.backend)) {
            return Err(PaintError::NoAdapter {
                looked_for: format!("{backends:?}"),
                detail: format!("the only adapter offered was {:?}", info.backend),
            });
        }

        // WebGL 2's limits, widened to whatever this adapter can actually address. Asking for the
        // baseline's *feature* bounds keeps one code path across five backends; taking the adapter's
        // real resolution is what stops a 4K page being refused on a machine that can draw it.
        let limits = wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
        let (device, queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("mjx-paint"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })) {
                Ok(pair) => pair,
                Err(error) => return Err(PaintError::Device(error.to_string())),
            };

        let complaints = Arc::new(Mutex::new(Vec::new()));
        let collector = Arc::clone(&complaints);
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            // `wgpu`'s default handler panics the process. A painter may not: a shader that does not
            // match its pipeline is a bug, but crashing the application a user is editing in is a
            // worse one, and a collected message can be reported.
            if let Ok(mut held) = collector.lock() {
                held.push(error.to_string());
            }
        }));

        // What multisampling this adapter will actually grant for the format we render into. Asked
        // rather than assumed, because a pipeline built at four samples against a format that
        // supports one is a validation error at draw time rather than at build time on some
        // backends, and because `report()` must state the count that was really used.
        let features = adapter.get_texture_format_features(OFFSCREEN_FORMAT);
        let sample_count = if features
            .flags
            .sample_count_supported(REQUESTED_SAMPLE_COUNT)
        {
            REQUESTED_SAMPLE_COUNT
        } else {
            1
        };

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            info,
            sample_count,
            complaints,
        })
    }

    /// The `wgpu` instance, for creating further surfaces.
    #[must_use]
    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    /// The device.
    #[must_use]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The queue.
    #[must_use]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// The adapter, for asking what a surface's preferred format is.
    #[must_use]
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    /// How many samples per pixel the pipelines are built at. `1` where multisampling was refused.
    #[must_use]
    pub const fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// The largest texture this device will make, on a side.
    #[must_use]
    pub fn max_texture_size(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }

    /// What is actually running.
    #[must_use]
    pub fn report(&self) -> BackendReport {
        BackendReport {
            api: match self.info.backend {
                wgpu::Backend::Vulkan => GraphicsApi::Vulkan,
                wgpu::Backend::Metal => GraphicsApi::Metal,
                wgpu::Backend::Dx12 => GraphicsApi::Direct3D12,
                wgpu::Backend::Gl => GraphicsApi::OpenGl,
                wgpu::Backend::BrowserWebGpu => GraphicsApi::WebGpu,
                // `Noop` is not compiled into this workspace's `wgpu`, so this arm is unreachable —
                // and it answers `None` rather than pretending, so that a build which somehow
                // enabled it would report a painter that draws nothing as exactly that.
                _ => GraphicsApi::None,
            },
            adapter: match self.info.device_type {
                wgpu::DeviceType::DiscreteGpu => AdapterKind::DiscreteGpu,
                wgpu::DeviceType::IntegratedGpu => AdapterKind::IntegratedGpu,
                wgpu::DeviceType::VirtualGpu => AdapterKind::VirtualGpu,
                wgpu::DeviceType::Cpu => AdapterKind::Cpu,
                wgpu::DeviceType::Other => AdapterKind::Other,
            },
            adapter_name: self.info.name.clone(),
            driver: self.info.driver_info.clone(),
            antialiasing: if self.sample_count > 1 {
                Antialiasing::Multisample(self.sample_count)
            } else {
                Antialiasing::None
            },
        }
    }

    /// Wait for everything submitted so far, then answer any driver complaint that arrived.
    ///
    /// # Errors
    ///
    /// [`PaintError::DeviceReported`] with every message the driver raised, joined.
    pub fn settle(&self) -> Result<(), PaintError> {
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        self.take_complaints()
    }

    /// Any driver complaint raised since the last time this was called.
    ///
    /// # Errors
    ///
    /// [`PaintError::DeviceReported`].
    pub fn take_complaints(&self) -> Result<(), PaintError> {
        let taken = match self.complaints.lock() {
            Ok(mut held) => core::mem::take(&mut *held),
            // A poisoned mutex means the collector's own closure panicked, which it cannot: it does
            // one `push`. Taking the contents anyway is strictly better than dropping the messages.
            Err(poisoned) => core::mem::take(&mut *poisoned.into_inner()),
        };
        if taken.is_empty() {
            return Ok(());
        }
        Err(PaintError::DeviceReported(taken.join("; ")))
    }
}

/// The backends that actually draw something, for this build and this target.
///
/// Everything the build enabled, minus anything that is not a real graphics stack. On `wasm32` that
/// is WebGPU and WebGL 2 — the opportunistic upgrade and the baseline — and `wgpu` prefers the
/// former when the browser has it, which is the selection policy MJXOFF-163 asks for and is
/// `wgpu`'s own.
#[must_use]
pub fn drawing_backends() -> wgpu::Backends {
    wgpu::Instance::enabled_backend_features() - wgpu::Backends::NOOP
}
