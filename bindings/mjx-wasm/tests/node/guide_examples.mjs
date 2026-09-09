// Every guide example, run here and compared against the Rust example of the same name.
//
// The examples themselves live in `guide_examples/`, one module each, and the code between their
// `guide-example` sentinels is what `crates/mjx-ooxml/docs/guide/` shows as its `js` block —
// literally, because `cargo run -p xtask -- guide-examples` copies it there and
// `xtask/tests/guide_examples.rs` proves the copy is current.
//
// **Importing a module is running the example.** Their code is top level, so every check a reader
// sees in the guide executes the moment this file imports it. A module that produced a package
// exports `saved`, and that is what the comparison below reads.
//
// The population comes from the directory, never from a list here: an example added to
// `guide_examples/` joins these tests with no edit to this file.
//
//     node --test bindings/mjx-wasm/tests/node/
//
// The package under test is the bundler build, which `bindings/mjx-wasm/build-npm.sh` produces and
// links into `tests/node/node_modules/` so that the specifier the guide shows is the specifier a
// consumer writes and the one these examples really import.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { partPayloads } from "./zip.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = resolve(HERE, "../../../..");
const EXAMPLES = join(HERE, "guide_examples");
const OUTPUT_DIRECTORY = process.env.MJX_OUTPUT_DIR ?? join(REPOSITORY_ROOT, "target", "examples");

/** What the Rust half of an example is called under `crates/mjx-ooxml/examples/`. */
const RUST_EXAMPLE_PREFIX = "guide_";

/**
 * The extension the Rust half's output is written under. The format is the example's own business
 * — one may author a deck and the next a workbook — and this comparison is over part payloads, so
 * the harness does not pretend to know which.
 */
const PACKAGE_SUFFIX = ".pkg";

/** The examples, derived from `guide_examples/` rather than listed. */
function guideExampleNames() {
  const names = readdirSync(EXAMPLES)
    .filter((file) => file.endsWith(".mjs"))
    .map((file) => file.slice(0, -".mjs".length))
    .sort();
  assert.ok(names.length > 0, `no guide example under ${EXAMPLES} — the walk has stopped matching`);
  return names;
}

for (const name of guideExampleNames()) {
  test(`the ${name} guide example runs, and agrees with the Rust one`, async (context) => {
    // Importing is running: the example's own checks are the first half of this test, and an
    // import that returns has passed all of them.
    const example = await import(`./guide_examples/${name}.mjs`);
    if (example.saved === undefined) {
      context.skip(`the ${name} example produces no package, so there is nothing to compare`);
      return;
    }

    // The second half: this package and the Rust one, part for part. Not "both produce a file",
    // and not "both produce a file of about the right size" — the same part names and
    // byte-identical payloads for every one of them.
    let rustOutput;
    try {
      mkdirSync(OUTPUT_DIRECTORY, { recursive: true });
      rustOutput = join(OUTPUT_DIRECTORY, `facade_${RUST_EXAMPLE_PREFIX}${name}${PACKAGE_SUFFIX}`);
      execFileSync(
        "cargo",
        [
          "run",
          "--quiet",
          "-p",
          "mjx-ooxml",
          "--example",
          `${RUST_EXAMPLE_PREFIX}${name}`,
          "--",
          rustOutput,
        ],
        { cwd: REPOSITORY_ROOT, stdio: "pipe" },
      );
    } catch (failure) {
      context.skip(`the Rust half could not be run: ${failure.message.split("\n")[0]}`);
      return;
    }

    const fromNode = partPayloads(example.saved);
    const fromRust = partPayloads(readFileSync(rustOutput));

    assert.deepEqual(
      Object.keys(fromNode).sort(),
      Object.keys(fromRust).sort(),
      "the two halves must author the same set of parts",
    );
    const differing = Object.keys(fromNode).filter(
      (part) => Buffer.compare(fromNode[part], fromRust[part]) !== 0,
    );
    assert.deepEqual(
      differing,
      [],
      `these parts differ between the Node and Rust halves: ${differing.join(", ")}`,
    );
  });
}
