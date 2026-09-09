/**
 * The arithmetic behind the icon bundle-size gate.
 *
 * MJXOFF-181's *"Done when"* line: **"Icon subsetting is asserted by bundle size — the built bundle
 * contains only referenced icons, and a test fails if the count exceeds the reference count. Prove
 * it can fail by importing the whole set."**
 *
 * There are two measurements here and they answer different questions.
 *
 * **`iconIdsIn` answers *which*.** Every glyph the subsetter emits is keyed by an id carrying the
 * `mjx-fluent:` sentinel, and those keys survive minification because a key containing `:` and `-`
 * cannot become a bare identifier. So the set of ids in the built assets can be compared with the
 * set the manifest asked for — and the gate asserts **equality**, not a ceiling. A subset that
 * silently shrank is as much a defect as one that grew: it means a component is referencing an icon
 * that is no longer there, which renders as a hole in a toolbar.
 *
 * **`svgPathBytesIn` answers *how much*.** Id equality alone would not notice 12 MB of Fluent SVG
 * arriving by some route that never touches our ids — a raw import, a copied file, a bundler
 * plugin. Measuring the path data catches that regardless of how it got in, which is what makes it
 * a *bundle-size* assertion rather than a manifest assertion wearing one's clothes.
 *
 * ## Why the budget has a baseline term
 *
 * Storybook's own interface ships icons, and they are in the same `assets/` directory as ours. A
 * budget of "three times our path data" would therefore be measuring somebody else's bundle and
 * failing. `storybookPathByteAllowance` is that contribution, measured once with generous headroom
 * and written down. If a Storybook upgrade moves it the gate fires and the number is re-measured —
 * a loud, one-line fix, and the alternative (no budget at all) is the thing the ticket calls a
 * vacuous gate.
 *
 * The margin that matters is the one at the other end: the whole vendor set is roughly **eight
 * megabytes** of path data, which is two orders of magnitude past this budget however the baseline
 * is tuned. `tests/icon-subset.test.ts` measures the real, whole set and watches the budget refuse
 * it — so the gate is known to be able to fail without anyone keeping a broken build.
 *
 * Data only, per U01's rule: the browser tier's specs run in Node and cannot import a module that
 * defines a custom element.
 */

/**
 * Storybook's own contribution of path-like strings to `storybook-static/assets`, in bytes.
 *
 * Measured at Storybook 10.6 with `npm run build-storybook`: **8.6 kB**, all of it the docs
 * renderer's own interface icons. The constant is set to four and a half times that so an ordinary
 * version bump does not trip it, and it is still two orders of magnitude below what the whole
 * vendor set would add. Re-measure if it ever fires — the gate's message says so.
 */
export const storybookPathByteAllowance = 40_000;

/**
 * How much of the subset's own path data the bundle is allowed to carry.
 *
 * Three, rather than one: the same drawing appears once in `generated.ts` and may legitimately be
 * duplicated across chunks by the bundler, and a source map or a dev-mode copy would add another.
 * Anything past that is not duplication, it is a second icon set.
 */
export const subsetDuplicationAllowance = 3;

/** The budget, in bytes, for a subset carrying `subsetPathBytes` of path data. */
export function iconPathByteBudget(subsetPathBytes: number): number {
  return storybookPathByteAllowance + subsetPathBytes * subsetDuplicationAllowance;
}

/**
 * Every `mjx-fluent:…` id occurring in a blob of source.
 *
 * The pattern deliberately stops at the character class the subsetter's ids are built from, so a
 * concatenated `…-20-regular"` in minified output yields the id and not the quote.
 */
export function iconIdsIn(source: string): Set<string> {
  const found = new Set<string>();
  for (const match of source.matchAll(/mjx-fluent:[a-z0-9-]+/g)) found.add(match[0]);
  return found;
}

/**
 * Total bytes of SVG path data in a blob of source.
 *
 * A path in a bundle is a quoted string that begins with a move command and then contains nothing
 * but path-command letters, numbers and separators. The 24-character floor keeps a stray `"M12"`
 * out; the alphabet is SVG's own, so a sentence of English prose cannot match — it would need to
 * contain no letter outside `aAcChHlLmMqQsStTvVzZeE`.
 *
 * ⚠ **All three quote characters, backtick included.** This was measured, not assumed: Storybook's
 * minifier rewrites `'M5 1a2 2 0…'` as a *template literal*, so a gate that looked only for
 * `'` and `"` reported **342 bytes** of path data in a bundle that carried fourteen thousand — a
 * budget that could never be exceeded, which is the identity-value trap wearing a regular
 * expression. The subset's own bytes being *found* is therefore itself asserted, in
 * `tests/browser/icons.spec.ts`, so this cannot silently regress to matching nothing.
 */
export function svgPathBytesIn(source: string): number {
  let total = 0;
  const pattern = /["'`]([Mm][-0-9.][-0-9.,\seEaAcChHlLmMqQsStTvVzZ]{22,})["'`]/g;
  for (const match of source.matchAll(pattern)) total += (match[1] ?? '').length + 1;
  return total;
}

/** What a failing budget should say, so the message names the number rather than a boolean. */
export function describeBudget(measured: number, budget: number): string {
  return (
    `${String(measured)} bytes of SVG path data in the built assets, against a budget of ` +
    `${String(budget)}. If this is a Storybook upgrade, re-measure ` +
    'dev/icon-budget.ts:storybookPathByteAllowance. If it is not, something has put icons into ' +
    'the bundle that src/icons/manifest.ts did not ask for — which is the whole thing this gate ' +
    'exists to prevent.'
  );
}
