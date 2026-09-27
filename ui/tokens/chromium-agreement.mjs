#!/usr/bin/env node
/**
 * **The gate that says there is one `color-mix(in srgb, …)` implementation, and that it is right.**
 *
 * MJXOFF-271 gave the token source a second tier: a derived colour is a `color-mix()` of the seeds
 * and knobs above it. Two things then evaluate that expression, and only one of them is ours:
 *
 * | Evaluator | Where | Reached by |
 * |---|---|---|
 * | `mjx_tokens::color_mix` | Rust | the generator (which resolved every committed value) and `Tokens::rederive` |
 * | Chromium's own | the browser | `derivations.css`, which states the expression literally |
 *
 * There is no third. `ui/tokens/tokens.ts` carries resolved colours rather than an algorithm, so
 * the TypeScript half of the platform has no `color-mix()` of its own to disagree with — and this
 * script asserts that the two that do exist agree, **for every derived token, in both colour
 * schemes**, against Chromium's `getComputedStyle`.
 *
 * ## Why this cannot be a unit test
 *
 * A unit test comparing our arithmetic against numbers we wrote down proves that we can copy. The
 * two behaviours this pipeline is most likely to get wrong are Chromium's, not ours:
 * `color-mix(in srgb, C p%, transparent)` is an alpha operation rather than a blend toward black,
 * and percentages that do not sum to 100% renormalise. Both are invisible to any test that does not
 * ask a browser.
 *
 * ## What it compares against
 *
 * The committed value in `tokens.css` — which is what the Rust generator wrote after evaluating the
 * derivation through `mjx_tokens::color_mix`, and which `crates/mjx-tokens/tests/artefacts_agree.rs`
 * separately proves is byte for byte the Rust table. So agreeing with `tokens.css` here *is*
 * agreeing with that one implementation.
 *
 * ## Two failability checks, because a green gate that cannot go red is not a gate
 *
 * 1. A deliberately wrong expectation must be reported as a disagreement.
 * 2. Overriding one seed must **move** the derived colours in the browser — which is also the
 *    proof that `derivations.css` does what it exists for: a host that sets `--theme-midground`
 *    re-themes every colour mixed from it, through the cascade, with no code.
 *
 * Run it with `node ui/tokens/chromium-agreement.mjs` (Playwright's Chromium, headless).
 */

import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium } from '@playwright/test';

const here = dirname(fileURLToPath(import.meta.url));
const tokensCss = readFileSync(resolve(here, 'tokens.css'), 'utf8');
const derivationsCss = readFileSync(resolve(here, 'derivations.css'), 'utf8');

/** Every literal `--name: value;` in a stylesheet, comments stripped and `var()` aliases skipped. */
function declarations(css) {
  const out = new Map();
  for (const match of css.replace(/\/\*[\s\S]*?\*\//g, '').matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)) {
    const value = match[2].trim();
    if (!value.startsWith('var(')) out.set(match[1], value);
  }
  return out;
}

/** Every scheme-relative alias `derivations.css` redeclares — the derived tier, by member name. */
function derivedAliases(css) {
  const out = [];
  for (const match of css.replace(/\/\*[\s\S]*?\*\//g, '').matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)) {
    out.push(match[1]);
  }
  return out;
}

const committed = declarations(tokensCss);
const derived = derivedAliases(derivationsCss);

if (derived.length === 0) {
  console.error('derivations.css declares nothing; there is no derived tier to check.');
  process.exit(1);
}

/**
 * The token property one scheme-relative alias stands for in a given scheme: `--theme-background`
 * in the dark scheme is `--theme-dark-background`. Derived here from first principles rather than
 * read out of the artefact, so a generator that emitted the wrong alias cannot make this agree.
 */
function tokenPropertyFor(alias, scheme) {
  const [group, ...member] = alias.replace(/^--/, '').split('-');
  return `--${group}-${scheme}-${member.join('-')}`;
}

/** `#rrggbb[aa]` → `[r, g, b, a]` with `a` in 0…1. */
function parseCommitted(hex) {
  const digits = hex.slice(1);
  const byte = (at) => parseInt(digits.slice(at, at + 2), 16);
  return [byte(0), byte(2), byte(4), digits.length === 8 ? byte(6) / 255 : 1];
}

/**
 * Chromium's own serialisation → `[r, g, b, a]`, with the channels still floating point.
 *
 * It emits `rgb(r, g, b)` / `rgba(r, g, b, a)` for a colour it can express that way and
 * `color(srgb r g b / a)` with fractional channels otherwise, and which one it picks has moved
 * between versions — so both are read rather than one assumed.
 */
function parseComputed(text) {
  const modern = /^color\(srgb\s+([\d.eE+-]+)\s+([\d.eE+-]+)\s+([\d.eE+-]+)(?:\s*\/\s*([\d.eE+-]+))?\)$/.exec(text);
  if (modern) {
    return [Number(modern[1]) * 255, Number(modern[2]) * 255, Number(modern[3]) * 255, modern[4] === undefined ? 1 : Number(modern[4])];
  }
  const legacy = /^rgba?\(([^)]+)\)$/.exec(text);
  if (legacy) {
    const parts = legacy[1].split(/[\s,/]+/).filter(Boolean).map(Number);
    return [parts[0], parts[1], parts[2], parts[3] === undefined ? 1 : parts[3]];
  }
  return undefined;
}

/**
 * The page. `tokens.css` first, `derivations.css` second — the order a host imports them in, and
 * the order that makes the derived tier an expression again rather than a frozen colour.
 *
 * Each probe paints `color:` from the alias, because a custom property is NOT resolved by
 * `getComputedStyle`: an unregistered one hands back its token stream with `var()` substituted and
 * the `color-mix()` still unevaluated. Putting it on a real property is what makes Chromium do the
 * arithmetic.
 */
function page(probes, overrides) {
  const inline = Object.entries(overrides ?? {})
    .map(([name, value]) => `${name}: ${value};`)
    .join(' ');
  return `<!doctype html><meta charset="utf-8">
<style>${tokensCss}\n${derivationsCss}\n${inline ? `:root { ${inline} }` : ''}</style>
<body>${probes.map((alias) => `<i id="${alias}" style="color: var(${alias})"></i>`).join('')}</body>`;
}

const disagreements = [];
const measured = [];

const browser = await chromium.launch();
try {
  for (const scheme of ['light', 'dark']) {
    const context = await browser.newContext({ colorScheme: scheme });
    const probe = await context.newPage();
    await probe.setContent(page(derived));
    // The explicit `data-theme` rather than the media query alone: `tokens.css` puts an explicit
    // choice above the system preference on purpose, and this is the path a host actually uses.
    await probe.evaluate((value) => {
      document.documentElement.dataset['theme'] = value;
    }, scheme);

    const computed = await probe.evaluate(
      (names) => Object.fromEntries(names.map((name) => [name, getComputedStyle(document.getElementById(name)).color])),
      derived,
    );

    for (const alias of derived) {
      const property = tokenPropertyFor(alias, scheme);
      const expectedText = committed.get(property);
      if (expectedText === undefined) {
        disagreements.push(`${scheme}: tokens.css declares no ${property}, which ${alias} stands for`);
        continue;
      }
      const expected = parseCommitted(expectedText);
      const actual = parseComputed(computed[alias]);
      if (actual === undefined) {
        disagreements.push(`${scheme}: Chromium reported ${alias} as ${computed[alias]}, which this script cannot read`);
        continue;
      }
      // Our table is eight bits a channel and Chromium's arithmetic is floating point, so the
      // assertion is that our byte is the CORRECT ROUNDING of Chromium's channel — not that the
      // two are within some tolerance of each other, which would let a real disagreement of one
      // step pass.
      for (const [at, channel] of ['red', 'green', 'blue'].entries()) {
        if (Math.round(actual[at]) !== expected[at]) {
          disagreements.push(
            `${scheme} · ${alias} (${property}): ${channel} is ${actual[at]} in Chromium and ` +
              `${expected[at]} in the committed artefacts (${expectedText} vs ${computed[alias]})`,
          );
        }
      }
      if (Math.round(actual[3] * 255) !== Math.round(expected[3] * 255)) {
        disagreements.push(
          `${scheme} · ${alias} (${property}): alpha is ${actual[3]} in Chromium and ${expected[3]} ` +
            `in the committed artefacts (${expectedText} vs ${computed[alias]})`,
        );
      }
      measured.push(`${scheme} · ${alias} = ${computed[alias]} = ${expectedText}`);
    }
    await context.close();
  }

  // ── Failability 1 · the comparison can report a disagreement ────────────────
  {
    const wrong = parseCommitted('#ff00ff');
    const first = derived[0];
    const context = await browser.newContext({ colorScheme: 'light' });
    const probe = await context.newPage();
    await probe.setContent(page([first]));
    const text = await probe.evaluate((name) => getComputedStyle(document.getElementById(name)).color, first);
    const actual = parseComputed(text);
    const agrees = [0, 1, 2].every((at) => Math.round(actual[at]) === wrong[at]);
    if (agrees) {
      disagreements.push('the comparison accepted magenta for a derived token, so it proves nothing');
    }
    await context.close();
  }

  // ── Failability 2 · a host seed override really does re-derive, in the browser ──
  {
    const context = await browser.newContext({ colorScheme: 'light' });
    const probe = await context.newPage();
    await probe.setContent(page(derived));
    const before = await probe.evaluate(
      (names) => Object.fromEntries(names.map((name) => [name, getComputedStyle(document.getElementById(name)).color])),
      derived,
    );
    await probe.evaluate(() => {
      // One seed, set the way a host sets it — inline on the root element, exactly as
      // hermes-universal's `applyTheme()` does.
      document.documentElement.style.setProperty('--theme-midground', '#0000ff');
      document.documentElement.style.setProperty('--theme-background-seed', '#0000ff');
    });
    const after = await probe.evaluate(
      (names) => Object.fromEntries(names.map((name) => [name, getComputedStyle(document.getElementById(name)).color])),
      derived,
    );
    const moved = derived.filter((alias) => before[alias] !== after[alias]);
    if (moved.length < derived.length / 2) {
      disagreements.push(
        `overriding two seeds moved only ${moved.length} of ${derived.length} derived tokens; ` +
          'derivations.css is not re-deriving through the cascade, which is the one thing it exists for',
      );
    }
    await context.close();
  }
} finally {
  await browser.close();
}

if (disagreements.length > 0) {
  console.error(
    `Chromium and mjx_tokens::color_mix disagree about ${disagreements.length} value(s).\n` +
      'One of the two implementations is wrong, and the four committed artefacts carry the Rust one.\n',
  );
  for (const line of disagreements) console.error(`  · ${line}`);
  process.exit(1);
}

console.log(
  `Chromium agrees with mjx_tokens::color_mix for all ${derived.length} derived tokens in both ` +
    `schemes (${measured.length} comparisons), and a seed override re-derives through the cascade.`,
);
for (const line of measured) console.log(`  ${line}`);
