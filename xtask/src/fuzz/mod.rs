//! `xtask fuzz` — the campaign against the untrusted-input entry points (MJXOFF-146).
//!
//! # Why this is here and not in a `fuzz/` crate
//!
//! `cargo-fuzz` was the obvious first answer and it is not available: a libFuzzer harness needs the
//! sanitizer flags, which need a nightly toolchain, and the toolchain this project builds and tests
//! on is stable. A harness only some machines can run is not a gate — it is a thing one person once
//! ran. This driver is stable Rust with no new dependency, so `cargo run -p xtask -- fuzz` works for
//! every developer and on CI, on demand.
//!
//! `xtask` is the right home for the same reason it hosts codegen: it is a host-only developer
//! binary, no shipped crate depends on it, and it is already above every crate it reaches into. The
//! alternative the ticket allows — a `fuzz/` member excluded from the default members — would mean
//! adding a `default-members` list to the workspace manifest purely to hide one crate, and would put
//! the campaign somewhere `cargo test --workspace` never compiles it. A dev-dependency on a library
//! crate was never an option: it would put the harness in the shipped graph's neighbourhood.
//!
//! # What it does
//!
//! A seeded mutational loop with **behavioural feedback**. Each execution is scored by a signature
//! folded from what the code under test actually did — which error it returned, how deep the tree
//! was, which node kinds appeared, whether the round trip held — and an input whose signature is new
//! joins the corpus. That is coverage feedback's job done without compiler instrumentation: it
//! cannot see a branch, but it can see a *behaviour*, and behaviour is what the properties here are
//! written against. The report prints corpus growth alongside the execution count precisely so a
//! campaign that explored nothing is visible as one.
//!
//! # The three ways an execution can fail, and how each is made loud
//!
//! * **A panic** is caught by [`std::panic::catch_unwind`] and recorded with its message and
//!   location, so one panicking input does not end the campaign.
//! * **Unbounded allocation** is measured by [`mjx_allocation_counter`]'s counting global
//!   allocator: the peak for each execution is compared against a soft ceiling, and a hard ceiling
//!   inside the allocator aborts rather than let the kernel decide.
//! * **A hang** is caught by a watchdog thread. Neither a panic hook nor an allocator can see an
//!   input that simply never returns, and without the watchdog it is indistinguishable from a slow
//!   campaign.
//!
//! Every input is written to the in-flight file *before* it runs, so the two failures that cannot
//! unwind — the hard ceiling and the watchdog — still name their cause.

mod container;
mod mutate;
mod random;
mod targets;

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::fuzz::random::Random;
use crate::fuzz::targets::{Finding, Target, TARGETS};

/// The counting allocator is installed for the whole `xtask` binary. Every other subcommand pays
/// two relaxed atomic operations per allocation for it, which is not measurable next to reading
/// schemas off disk, and the alternative — a second binary — would be a second thing to keep in
/// step.
///
/// The implementation lives in `mjx-allocation-counter` rather than here because MJXOFF-95 needs the
/// same instrument from a `mjx-sml` test binary, and nothing may depend on `xtask`. One
/// `unsafe impl GlobalAlloc` in the workspace, two consumers.
#[global_allocator]
static ALLOCATOR: mjx_allocation_counter::Counting = mjx_allocation_counter::Counting;

/// How much a single execution may have allocated at its peak before it is reported as a finding.
///
/// Inputs are capped at [`mutate::MAXIMUM_INPUT`] (64 KiB), so 16 MiB is a factor of 256 of
/// headroom over the largest input the campaign can build. A parser that needs more than that for a
/// 64 KiB input is amplifying, which is the defect this ceiling exists to name.
const SOFT_CEILING: usize = 16 * 1024 * 1024;

/// The point at which the allocator stops the process instead of letting the kernel do it.
const HARD_CEILING: usize = 512 * 1024 * 1024;

/// How long one execution may take before the watchdog calls it a hang.
const EXECUTION_BUDGET: Duration = Duration::from_secs(20);

/// Defaults chosen so `cargo run -p xtask -- fuzz` is a useful campaign without arguments.
const DEFAULT_ITERATIONS: u64 = 200_000;
const DEFAULT_SEED: u64 = 0x6D6A_785F_6F6F_786D;

/// The largest corpus the campaign will keep per target, so the corpus itself cannot become the
/// memory problem it is looking for.
const MAXIMUM_CORPUS: usize = 4_096;

/// Where the driver writes the input it is about to run, and the inputs that failed.
struct Workspace {
    in_flight: PathBuf,
    findings: PathBuf,
}

impl Workspace {
    fn create(root: &Path) -> Result<Self> {
        let base = root.join("target/fuzz");
        let findings = base.join("findings");
        std::fs::create_dir_all(&findings)
            .with_context(|| format!("creating {}", findings.display()))?;
        Ok(Self {
            in_flight: base.join("in-flight.bin"),
            findings,
        })
    }
}

/// What the operator asked for.
#[derive(Debug)]
struct Options {
    iterations: u64,
    seed: u64,
    selected: Option<String>,
    time_budget: Option<Duration>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            iterations: DEFAULT_ITERATIONS,
            seed: DEFAULT_SEED,
            selected: None,
            time_budget: None,
        }
    }
}

fn parse_options(arguments: &[String]) -> Result<Options> {
    let mut options = Options::default();
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        let mut value = || {
            rest.next()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("{argument} needs a value"))
        };
        match argument.as_str() {
            "--iterations" => options.iterations = value()?.parse()?,
            "--seed" => options.seed = value()?.parse()?,
            "--target" => options.selected = Some(value()?),
            "--seconds" => options.time_budget = Some(Duration::from_secs(value()?.parse()?)),
            "--list" => {
                for target in TARGETS {
                    println!("{:<18} {}", target.name, target.entry_point);
                }
                std::process::exit(0);
            }
            other => bail!(
                "unknown fuzz argument {other:?}. Use --target NAME, --iterations N, --seed N, \
                 --seconds N, --list"
            ),
        }
    }
    if let Some(name) = &options.selected {
        if !targets::names().contains(name.as_str()) {
            bail!("unknown target {name:?}; `--list` names them all");
        }
    }
    Ok(options)
}

/// The last panic a caught execution produced, captured by the hook so `catch_unwind`'s opaque
/// payload does not have to be interpreted.
static LAST_PANIC: Mutex<Option<String>> = Mutex::new(None);

/// Bumped once per execution; the watchdog watches it stop moving.
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// Milliseconds since the campaign started, as of the current execution's start.
static STARTED_AT: AtomicU64 = AtomicU64::new(0);
/// Set while an execution is in flight, so the watchdog does not fire during setup or reporting.
static IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// Runs the campaign.
///
/// # Errors
/// If the arguments are malformed or the workspace directory cannot be created.
pub fn run(arguments: &[String]) -> Result<()> {
    let options = parse_options(arguments)?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let workspace = Workspace::create(&root)?;

    mjx_allocation_counter::set_hard_ceiling(
        HARD_CEILING,
        "See the in-flight input file the driver wrote before this execution began.",
    );
    install_panic_hook();
    let campaign_start = Instant::now();
    start_watchdog(campaign_start, workspace.in_flight.clone());

    let chosen: Vec<&Target> = TARGETS
        .iter()
        .filter(|target| {
            options
                .selected
                .as_deref()
                .is_none_or(|name| name == target.name)
        })
        .collect();

    println!(
        "fuzz: {} target(s), {} iterations each, seed {}, soft ceiling {} MiB, hard ceiling {} MiB",
        chosen.len(),
        options.iterations,
        options.seed,
        SOFT_CEILING / (1024 * 1024),
        HARD_CEILING / (1024 * 1024),
    );

    let mut reports = Vec::new();
    for target in chosen {
        reports.push(run_target(
            target,
            &options,
            &workspace,
            campaign_start,
            options.time_budget,
        )?);
    }

    report(&reports, campaign_start.elapsed());
    let failed: usize = reports.iter().map(|report| report.findings.len()).sum();
    if failed > 0 {
        bail!("{failed} finding(s) — see target/fuzz/findings/");
    }
    Ok(())
}

/// What one target's campaign produced.
struct TargetReport {
    name: &'static str,
    executions: u64,
    seeds: usize,
    corpus: usize,
    signatures: usize,
    peak: usize,
    elapsed: Duration,
    findings: Vec<(Finding, PathBuf)>,
}

fn run_target(
    target: &Target,
    options: &Options,
    workspace: &Workspace,
    campaign_start: Instant,
    time_budget: Option<Duration>,
) -> Result<TargetReport> {
    let started = Instant::now();
    let mut random = Random::new(options.seed ^ fnv(target.name));
    let mut corpus = (target.seeds)();
    let seeds = corpus.len();
    let mut signatures: HashSet<u64> = HashSet::new();
    let mut findings = Vec::new();
    let mut peak = 0usize;
    let mut executions = 0u64;
    let mut recorded: HashSet<&'static str> = HashSet::new();

    // The seeds run first, unmutated. A seed that already violates a property should be reported as
    // itself rather than as whatever the mutator later made of it.
    let mut pending: Vec<Vec<u8>> = corpus.clone();

    while executions < options.iterations {
        let input = match pending.pop() {
            Some(input) => input,
            None => {
                let Some(parent) = choose_parent(&corpus, &mut random) else {
                    break;
                };
                mutate::mutate(&parent, &mut random, &corpus)
            }
        };

        std::fs::write(&workspace.in_flight, &input)
            .with_context(|| format!("writing {}", workspace.in_flight.display()))?;

        STARTED_AT.store(elapsed_millis(campaign_start), Ordering::Relaxed);
        GENERATION.fetch_add(1, Ordering::Relaxed);
        IN_FLIGHT.store(true, Ordering::Relaxed);
        let before = mjx_allocation_counter::reset_peak();
        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (target.run)(&input)));
        let used = mjx_allocation_counter::peak().saturating_sub(before);
        IN_FLIGHT.store(false, Ordering::Relaxed);
        executions += 1;
        peak = peak.max(used);

        let mut faults: Vec<Finding> = Vec::new();
        match outcome {
            Err(_) => faults.push(Finding {
                kind: "panic",
                detail: LAST_PANIC
                    .lock()
                    .ok()
                    .and_then(|mut slot| slot.take())
                    .unwrap_or_else(|| "a panic with no message".to_owned()),
            }),
            Ok(outcome) => {
                if signatures.insert(outcome.signature()) && corpus.len() < MAXIMUM_CORPUS {
                    corpus.push(input.clone());
                }
                faults.extend(outcome.findings);
            }
        }
        if used > SOFT_CEILING {
            faults.push(Finding {
                kind: "allocation",
                detail: format!(
                    "{used} bytes at peak for a {}-byte input, over the {SOFT_CEILING}-byte ceiling",
                    input.len()
                ),
            });
        }

        for fault in faults {
            // One saved input per kind: a hundred copies of the same defect is a hundred files to
            // read and one thing to fix.
            if !recorded.insert(fault.kind) {
                continue;
            }
            let path = workspace
                .findings
                .join(format!("{}-{}.bin", target.name, fault.kind));
            std::fs::write(&path, &input).with_context(|| format!("writing {}", path.display()))?;
            eprintln!(
                "fuzz: {} — {} — {} (input saved to {})",
                target.name,
                fault.kind,
                fault.detail,
                path.display()
            );
            findings.push((fault, path));
        }

        if time_budget.is_some_and(|budget| started.elapsed() >= budget) {
            break;
        }
    }

    Ok(TargetReport {
        name: target.name,
        executions,
        seeds,
        corpus: corpus.len(),
        signatures: signatures.len(),
        peak,
        elapsed: started.elapsed(),
        findings,
    })
}

/// How many of the most recently kept inputs count as the frontier.
const FRONTIER: usize = 64;

/// Chooses the input to mutate next.
///
/// Half the time from the **frontier** — the most recently kept entries, which are by construction
/// the ones that most recently did something the campaign had not seen — and half uniformly, so
/// nothing in the corpus is ever abandoned.
///
/// This is the one place a black-box campaign can imitate what coverage feedback buys a real fuzzer.
/// Uniform selection over a corpus of hundreds means an input that just reached new behaviour is
/// picked once in hundreds of executions, and its neighbourhood — where the next new behaviour is —
/// goes unexplored. With the planted defect in place, uniform selection did not find it in 300,000
/// executions; this and the short-length bias together found it.
fn choose_parent(corpus: &[Vec<u8>], random: &mut Random) -> Option<Vec<u8>> {
    if corpus.is_empty() {
        return None;
    }
    if random.below(2) == 0 {
        let frontier = &corpus[corpus.len().saturating_sub(FRONTIER)..];
        return random.pick(frontier).cloned();
    }
    random.pick(corpus).cloned()
}

fn report(reports: &[TargetReport], total: Duration) {
    println!("\n{:-<96}", "");
    println!(
        "{:<18} {:>10} {:>7} {:>8} {:>10} {:>10} {:>9}  findings",
        "target", "execs", "seeds", "corpus", "behaviours", "peak KiB", "seconds"
    );
    for report in reports {
        println!(
            "{:<18} {:>10} {:>7} {:>8} {:>10} {:>10} {:>9.1}  {}",
            report.name,
            report.executions,
            report.seeds,
            report.corpus,
            report.signatures,
            report.peak / 1024,
            report.elapsed.as_secs_f64(),
            if report.findings.is_empty() {
                "none".to_owned()
            } else {
                report
                    .findings
                    .iter()
                    .map(|(finding, _)| finding.kind)
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );
    }
    println!("{:-<96}", "");
    println!("total {:.1}s", total.as_secs_f64());
}

/// Records each panic's message and location instead of printing a backtrace per execution.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let where_ = info
            .location()
            .map_or_else(|| "unknown location".to_owned(), ToString::to_string);
        let what = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "a panic with no message".to_owned());
        if let Ok(mut slot) = LAST_PANIC.lock() {
            *slot = Some(format!("{what} (at {where_})"));
        }
    }));
}

/// The hang detector.
///
/// A panic hook cannot see an input that never returns and neither can the allocator. Without this,
/// an infinite loop reads as a slow campaign and the operator waits. The thread aborts the process,
/// which is loud, and the in-flight file already holds the input that did it.
fn start_watchdog(campaign_start: Instant, in_flight: PathBuf) {
    std::thread::spawn(move || {
        let mut last_generation = u64::MAX;
        let mut last_seen = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(250));
            let generation = GENERATION.load(Ordering::Relaxed);
            if generation != last_generation {
                last_generation = generation;
                last_seen = Instant::now();
                continue;
            }
            if !IN_FLIGHT.load(Ordering::Relaxed) {
                last_seen = Instant::now();
                continue;
            }
            if last_seen.elapsed() > EXECUTION_BUDGET {
                let began = STARTED_AT.load(Ordering::Relaxed);
                let _ = std::io::stderr().write_all(
                    format!(
                        "\nfuzz: execution {generation} has not returned in {}s (it began {}ms into \
                         the campaign, which has run {:.1}s). The input is in {}. Aborting.\n",
                        EXECUTION_BUDGET.as_secs(),
                        began,
                        campaign_start.elapsed().as_secs_f64(),
                        in_flight.display(),
                    )
                    .as_bytes(),
                );
                std::process::abort();
            }
        }
    });
}

fn elapsed_millis(since: Instant) -> u64 {
    u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// FNV-1a over a name, so each target's generator starts somewhere different from one `--seed`.
fn fnv(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::{parse_options, targets};

    #[test]
    fn the_defaults_run_every_target() {
        let options = parse_options(&[]).expect("no arguments is a valid campaign");
        assert!(options.selected.is_none());
        assert!(options.iterations > 0);
    }

    #[test]
    fn an_unknown_target_is_refused_rather_than_silently_running_none() {
        // Silently matching no target would print a clean report having executed nothing, which is
        // exactly the false pass this campaign is built to avoid.
        let arguments = ["--target".to_owned(), "not-a-target".to_owned()];
        assert!(parse_options(&arguments).is_err());
        let arguments = ["--target".to_owned(), "xml-fidelity".to_owned()];
        assert!(parse_options(&arguments).is_ok());
        assert!(targets::names().contains("xml-fidelity"));
    }
}
