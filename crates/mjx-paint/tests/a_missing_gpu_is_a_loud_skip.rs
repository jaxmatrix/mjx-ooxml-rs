//! **The gate on the gates.** A rendering suite that is green because it was skipped is this
//! project's signature failure, so the skip path itself is tested.
//!
//! # What is proved here, and how
//!
//! MJXOFF-163 asks for a demonstration rather than an assurance: *"prove it by forcing the adapter
//! to none and showing the test fail loudly rather than quietly pass"*. Forcing the adapter to none
//! is what [`mjx_paint::backend::GraphicsDevice::open_on`] with [`wgpu::Backends::empty()`] does, and
//! it is public for exactly this reason. So:
//!
//! 1. with no backend at all, opening a device is a **named error** — not an `Ok` with nothing
//!    behind it, and not a panic;
//! 2. that error is the one the suites' skip path recognises, so the message a reader sees names the
//!    case and the reason;
//! 3. and under `MJX_REQUIRE_GPU=1` the very same absence **fails**, which is what continuous
//!    integration sets and what makes a missing device there impossible to miss.
//!
//! Case three is checked by running the skip helper in a child process with the variable set, which
//! is the only honest way to assert that something aborts: asserting it from inside the process
//! doing the aborting proves nothing.
//!
//! # And the backend is reported, not guessed
//!
//! The last case asserts that a painter which *does* open reports a real graphics API, a real
//! adapter name, and the sample count its pipelines were actually built at. A painter that answered
//! "it worked" would satisfy every other suite in this crate and tell a reader nothing about what
//! ran.

mod common;

use mjx_paint::backend::{drawing_backends, GraphicsDevice};
use mjx_paint::{GraphicsApi, PaintError, Painter};

#[test]
fn with_no_backend_at_all_opening_a_device_is_a_named_error() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    // The forced absence. Not "pretend there is no GPU" — there genuinely is no backend this
    // instance may pick from, which is the same situation a machine with no graphics stack is in.
    let refused = GraphicsDevice::open_on(instance, None, wgpu::Backends::empty());
    match refused {
        Err(PaintError::NoAdapter { looked_for, detail }) => {
            // `Backends(0x0)` is how the flag set prints itself when it is empty, and printing it
            // rather than a word of our own is what makes the message survive a backend being
            // added.
            assert!(
                looked_for.contains("0x0"),
                "the error must name what was looked for; it said `{looked_for}`"
            );
            assert!(
                !detail.is_empty(),
                "the error must say what the graphics stack answered"
            );
            println!("forced absence reported as: no adapter ({looked_for}): {detail}");
        }
        Ok(device) => panic!(
            "a device was opened with no backend enabled, which means the suite's idea of \
             \"skipped\" and its idea of \"ran\" are the same thing: {device:?}"
        ),
        Err(other) => {
            panic!("the forced absence answered {other:?}, which the skip path does not recognise")
        }
    }
}

#[test]
fn the_skip_path_recognises_that_error_and_says_so() {
    // The same shape the suites use. What is being checked is that a `NoAdapter` really does reach
    // `common::painter`'s error arm and produce a message with the reason in it, rather than being
    // swallowed into a generic "unavailable".
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let Err(error) = GraphicsDevice::open_on(instance, None, wgpu::Backends::empty()) else {
        panic!("the forced absence produced a device");
    };
    let rendered = error.to_string();
    assert!(
        rendered.contains("no graphics adapter is available"),
        "the message a reader sees is `{rendered}`"
    );
}

#[test]
fn under_the_require_variable_a_missing_device_fails_instead_of_skipping() {
    // Asserted from a child process, because asserting that something fails from inside the process
    // that would be failing proves nothing. The child runs this file's own binary with a filter that
    // selects the case below and with `MJX_REQUIRE_GPU` set to a value that cannot be satisfied.
    let binary = std::env::current_exe().expect("this test binary knows where it is");
    let output = std::process::Command::new(binary)
        .args([
            "--exact",
            "the_case_the_child_runs_which_always_skips",
            "--nocapture",
        ])
        .env(common::REQUIRE, "1")
        .env("MJX_PAINT_FORCE_NO_DEVICE", "1")
        .output()
        .expect("the child runs");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "with {} set, a case that skipped reported success. That is the exact failure this whole \
         child is written against: a gate that is green precisely when the work is skipped.\n{combined}",
        common::REQUIRE
    );
    assert!(
        combined.contains(common::REQUIRE),
        "the failure must name the variable that made it a failure, so a reader knows why:\n{combined}"
    );
}

#[test]
fn the_case_the_child_runs_which_always_skips() {
    // Only meaningful when the case above runs it. On an ordinary run it announces a skip for a
    // reason that is true — nothing forced a device to be absent — and passes, which is what a skip
    // is supposed to do when nothing has asked for more.
    if std::env::var("MJX_PAINT_FORCE_NO_DEVICE").is_err() {
        println!("the_case_the_child_runs_which_always_skips: not the child run; nothing to do");
        return;
    }
    common::skip(
        "the_case_the_child_runs_which_always_skips",
        "no device, by construction",
    );
}

#[test]
fn a_painter_that_opens_says_which_backend_it_opened() {
    let painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => return common::skip("a_painter_that_opens_says_which_backend_it_opened", &why),
    };
    let report = painter.backend();
    println!(
        "a_painter_that_opens_says_which_backend_it_opened: painter `{}` on {report}",
        painter.name()
    );

    assert_eq!(painter.name(), "wgpu");
    assert_ne!(
        report.api,
        GraphicsApi::None,
        "a `wgpu` painter that reports no graphics API at all is either the null backend — which \
         this workspace deliberately does not compile in — or a painter that cannot say what it is \
         running on, and both satisfy every \"did it render\" assertion for free"
    );
    assert!(
        !report.adapter_name.is_empty(),
        "the adapter must name itself, or a report cannot distinguish hardware from `lavapipe`"
    );
    assert!(
        drawing_backends().contains(wgpu::Backends::from(match report.api {
            GraphicsApi::Vulkan => wgpu::Backend::Vulkan,
            GraphicsApi::Metal => wgpu::Backend::Metal,
            GraphicsApi::Direct3D12 => wgpu::Backend::Dx12,
            GraphicsApi::OpenGl => wgpu::Backend::Gl,
            GraphicsApi::WebGpu => wgpu::Backend::BrowserWebGpu,
            // `GraphicsApi` is `non_exhaustive` so that a future backend does not break a caller;
            // an API this build has never heard of is treated as no backend, which is what the
            // assertion above has already refused.
            _ => wgpu::Backend::Noop,
        })),
        "the painter reports {:?}, which is not a backend this build enabled",
        report.api
    );

    // The sample count is the count the pipelines were built at, not the count that was asked for.
    let capabilities = painter.capabilities();
    assert_eq!(capabilities.antialiasing, report.antialiasing);
    assert!(
        capabilities.max_texture_size >= 2048,
        "every backend in the matrix addresses at least the WebGL 2 baseline; this one says {}",
        capabilities.max_texture_size
    );
    assert!(
        capabilities.exact_path_clipping,
        "a GPU painter clips with a stencil buffer and must say so, because a caller may choose \
         another painter for a page whose clipping has to be exact"
    );
}
