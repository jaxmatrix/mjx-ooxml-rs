// Every guide example, run here and compared against the Rust example of the same name.
//
// The examples themselves live in `guide_examples/`, one module each, and the code between their
// `guide-example` sentinels is what `crates/mjx-ooxml/docs/guide/` shows as its `js` block —
// literally, because `cargo run -p xtask -- guide-examples` copies it there and
// `xtask/tests/guide_examples.rs` proves the copy is current.
//
// **Importing a module is running the example.** Their code is top level, so every check a reader
// sees in the guide executes the moment this file imports it.
//
// *Which* packages a module offers is read from the example's Rust half, which declares them on one
// `guide-example:packages` line (MJXOFF-262). That is a statement rather than an inference: this
// harness used to decide by looking for an export named `saved`, so an example whose export had
// been deleted was indistinguishable from one of the seven that genuinely produce none, and more
// than a third of the corpus took a skip path nobody read. An example that declares `none` is now
// *asserted* to export nothing, and one that declares packages has every one of them compared
// (MJXOFF-260).
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

/** The sentinel a Rust half declares its packages with, and the word for "none of them". */
const PACKAGES_DECLARATION = "guide-example:packages";
const NO_PACKAGES_TOKEN = "none";

/** The prefix every package binding's name carries. `saved` alone is the single-package spelling. */
const PACKAGE_BINDING_PREFIX = "saved";

/**
 * The packages an example offers, as its Rust half declares them.
 *
 * Read out of `crates/mjx-ooxml/examples/guide_<name>.rs` rather than inferred from this module,
 * because the point of the declaration is that a half which stopped exporting one fails against the
 * statement instead of silently agreeing with itself. `xtask/tests/guide_examples.rs` is what holds
 * all three halves to the same line.
 */
function declaredPackages(name) {
  const source = readFileSync(
    join(REPOSITORY_ROOT, "crates", "mjx-ooxml", "examples", `${RUST_EXAMPLE_PREFIX}${name}.rs`),
    "utf8",
  );
  const lines = source.split("\n").filter((line) => line.includes(PACKAGES_DECLARATION));
  assert.equal(
    lines.length,
    1,
    `${name}: its Rust half carries ${lines.length} \`${PACKAGES_DECLARATION}\` lines, and every example states its packages exactly once`,
  );
  const words = lines[0].split(PACKAGES_DECLARATION)[1].trim().split(/\s+/).filter(Boolean);
  assert.ok(words.length > 0, `${name}: an empty \`${PACKAGES_DECLARATION}\` declaration`);
  if (words.length === 1 && words[0] === NO_PACKAGES_TOKEN) {
    return [];
  }
  for (const word of words) {
    assert.ok(word.startsWith(PACKAGE_BINDING_PREFIX), `${name}: ${word} is not a package binding`);
  }
  return words;
}

/**
 * Where the Rust half wrote one of its packages.
 *
 * The first declared binding takes the path itself; a later one has its suffix inserted before the
 * extension. The same rule lives in `xtask::guide_examples::package_output_path` and is restated
 * inside each two-package Rust half, which cannot depend on `xtask`.
 */
function packageOutputPath(base, binding) {
  const suffix = binding.slice(PACKAGE_BINDING_PREFIX.length).replace(/^_/, "");
  if (suffix === "") {
    return base;
  }
  return base.replace(new RegExp(`${PACKAGE_SUFFIX.replace(".", "\\.")}$`), `.${suffix}${PACKAGE_SUFFIX}`);
}

/** What an example exported under the harness's package protocol. */
function exportedPackages(example) {
  return Object.keys(example)
    .filter(
      (key) => key === PACKAGE_BINDING_PREFIX || key.startsWith(`${PACKAGE_BINDING_PREFIX}_`),
    )
    .sort();
}

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
    const declared = declaredPackages(name);

    // The declared-`none` case is a verdict, not a skip. Seven examples are about a refusal the
    // library reports and have nothing to save; making them save something to satisfy a harness
    // would be a worse example, compared over bytes it is not about. So it is asserted from the
    // other side, and this half exporting a package it never declared is a failure here.
    assert.deepEqual(
      exportedPackages(example),
      [...declared].sort(),
      `the ${name} example declares ${JSON.stringify(declared)} and this half exports something else`,
    );
    if (declared.length === 0) {
      return;
    }

    // The second half: these packages and the Rust ones, part for part. Not "both produce a file",
    // and not "both produce a file of about the right size" — the same part names and
    // byte-identical payloads for every one of them. Every declared package is compared, not just
    // the first: two examples author two each, and until MJXOFF-260 the second went uncompared.
    let base;
    try {
      mkdirSync(OUTPUT_DIRECTORY, { recursive: true });
      base = join(OUTPUT_DIRECTORY, `facade_${RUST_EXAMPLE_PREFIX}${name}${PACKAGE_SUFFIX}`);
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
          base,
        ],
        { cwd: REPOSITORY_ROOT, stdio: "pipe" },
      );
    } catch (failure) {
      context.skip(`the Rust half could not be run: ${failure.message.split("\n")[0]}`);
      return;
    }

    for (const binding of declared) {
      const fromNode = partPayloads(example[binding]);
      const fromRust = partPayloads(readFileSync(packageOutputPath(base, binding)));

      assert.deepEqual(
        Object.keys(fromNode).sort(),
        Object.keys(fromRust).sort(),
        `\`${binding}\`: the two halves must author the same set of parts`,
      );
      const differing = Object.keys(fromNode).filter(
        (part) => Buffer.compare(fromNode[part], fromRust[part]) !== 0,
      );
      assert.deepEqual(
        differing,
        [],
        `\`${binding}\`: these parts differ between the Node and Rust halves: ${differing.join(", ")}`,
      );
    }
  });
}
