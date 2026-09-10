import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

import {
  fluentFileName,
  iconIdPrefix,
  iconRequests,
  iconSizes,
  iconVariants,
  requestedIconIds,
  type IconSize,
  type IconVariant,
} from '../src/icons/manifest.ts';
import {
  generatedIconIdPrefix,
  iconGlyphs,
  subsetGlyphCount,
  subsetPathBytes,
} from '../src/icons/generated.ts';
import {
  describeBudget,
  iconIdsIn,
  iconPathByteBudget,
  svgPathBytesIn,
} from '../dev/icon-budget.ts';

/**
 * The icon subset, and the proof that its gate can reject.
 *
 * MJXOFF-181's trap: *"An icon subset is satisfied by any SVG. The gate must be about the subset
 * being the right one and staying it — assert the set, and that an icon not in the subset cannot
 * be referenced."*
 *
 * The bundle half of that lives in `tests/browser/icons.spec.ts`, because it needs a build. What
 * is here is the half that needs no browser, and the one thing neither of them could do on its
 * own: **measure the whole vendor set and watch the budget refuse it.** The ticket asks to *"prove
 * it can fail by importing the whole set"*, and a test that imports the whole set into the
 * catalogue would leave the catalogue broken. Measuring the real 20,679 files here proves the same
 * thing permanently, on every run, with nothing broken.
 */

const vendorRoot = resolve(import.meta.dirname, '../node_modules/@fluentui/svg-icons/icons');

describe('the manifest', () => {
  it('asks only for sizes and variants that exist', () => {
    for (const request of iconRequests) {
      expect(request.sizes.length, `${request.name} asks for no sizes`).toBeGreaterThan(0);
      expect(request.variants.length, `${request.name} asks for no variants`).toBeGreaterThan(0);
      expect(request.why.trim(), `${request.name} has no reason`).not.toBe('');
      for (const size of request.sizes) expect(iconSizes).toContain(size);
      for (const variant of request.variants) expect(iconVariants).toContain(variant);
    }
  });

  it('names each icon once', () => {
    const names = iconRequests.map((request) => request.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it('carries one icon at every size, so the size ladder has something to render', () => {
    // Foundations/Icons · The Size Ladder renders whichever icon this is. Without one, that story
    // silently degrades to a single box and the ladder is never exercised — the identity-value
    // trap in its typographic form, one size standing in for a scale.
    const ladder = iconRequests.filter((request) => request.sizes.length === iconSizes.length);
    expect(ladder.length).toBeGreaterThan(0);
  });
});

describe('the generated subset', () => {
  it('agrees with the manifest exactly, in both directions', () => {
    const requested = [...requestedIconIds()].sort();
    const generated = Object.keys(iconGlyphs).sort();
    expect(generated).toEqual(requested);
    expect(subsetGlyphCount).toBe(requested.length);
  });

  it('restates the sentinel the bundle gate greps for', () => {
    expect(generatedIconIdPrefix).toBe(iconIdPrefix);
  });

  it('draws every glyph in the box its id claims', () => {
    for (const [id, glyph] of Object.entries(iconGlyphs)) {
      const size = /-(\d+)-(?:regular|filled)$/.exec(id)?.[1];
      expect(size, `${id} has no size in its id`).toBeDefined();
      expect(glyph.viewBox, `${id} is drawn in the wrong box`).toBe(`0 0 ${String(size)} ${String(size)}`);
      expect(glyph.paths.length, `${id} has no paths`).toBeGreaterThan(0);
    }
  });

  it('carries no fill, so every icon is tintable from a token', () => {
    // The whole reason `<mjx-icon>` can be coloured by the text around it. A vendor `_color`
    // variant would break it, and the generator refuses one — this is that refusal, checked from
    // the other end.
    for (const [id, glyph] of Object.entries(iconGlyphs)) {
      for (const path of glyph.paths) {
        expect(path, `${id} carries markup, not path data`).not.toMatch(/[<>]/);
      }
    }
  });

  it('reports its own path-data size honestly', () => {
    const measured = Object.values(iconGlyphs).reduce(
      (total, glyph) => total + glyph.paths.reduce((sum, path) => sum + path.length, 0),
      0,
    );
    expect(subsetPathBytes).toBe(measured);
  });

  // Sixty seconds for the same reason as the whole-set test below: it reads 138 vendor files, and
  // vitest's five-second default is a measurement of the machine rather than of the code.
  it('is what the vendor actually draws, byte for byte', { timeout: 60_000 }, () => {
    // The generator could have transformed the paths — optimised, rounded, re-emitted. It does
    // not, and this is what says so: every path in the subset is the vendor's own string. An icon
    // that had been "cleaned up" on the way in would be a drawing this project maintains rather
    // than one Microsoft maintains, which is the opposite of the reason Fluent was chosen.
    for (const request of iconRequests) {
      for (const size of request.sizes) {
        for (const variant of request.variants) {
          const file = fluentFileName(request.name, size as IconSize, variant as IconVariant);
          const source = readFileSync(resolve(vendorRoot, file), 'utf8');
          const glyph = iconGlyphs[`${iconIdPrefix}${request.name}-${String(size)}-${variant}`];
          expect(glyph, `${file} is missing from the subset`).toBeDefined();
          for (const path of glyph?.paths ?? []) expect(source).toContain(`d="${path}"`);
        }
      }
    }
  });
});

describe('the bundle budget, proved able to reject', () => {
  it('accepts the subset it was derived from', () => {
    const asABundle = JSON.stringify(iconGlyphs);
    expect(svgPathBytesIn(asABundle)).toBeLessThanOrEqual(iconPathByteBudget(subsetPathBytes));
    expect(iconIdsIn(asABundle)).toEqual(new Set(requestedIconIds()));
  });

  // ⚠ Ten minutes, and the number is measured rather than guessed. This test reads 20,679 files
  // off disk: **0.8 seconds** on an idle machine, and **over sixty** while a
  // `cargo test --workspace` is running in the same checkout, which is the ordinary condition in
  // this repository. It timed out exactly that way on MJXOFF-181's first full gate run.
  //
  // A resource failure reads exactly like a code failure, and the code failure this test is
  // supposed to report — *the budget stopped rejecting the whole vendor set* — is the most
  // important one in the child. A default timeout would have made it the flakiest, and a flaky
  // gate is one nobody reads.
  it('refuses the whole vendor set, measured rather than imagined', { timeout: 600_000 }, () => {
    // ⚠ This is the ticket's *"prove it can fail by importing the whole set"*, done in the one
    // place where importing the whole set costs nothing: the real 20,679 files are read, their
    // real path data is arranged the way a bundler would arrange it, and the real budget function
    // is asked. Nothing is simulated and nothing stays broken afterwards.
    const files = readdirSync(vendorRoot).filter((name) => name.endsWith('.svg'));
    expect(files.length).toBeGreaterThan(20_000);

    const paths: string[] = [];
    for (const file of files) {
      const source = readFileSync(resolve(vendorRoot, file), 'utf8');
      for (const match of source.matchAll(/<path d="([^"]+)"/g)) paths.push(match[1] ?? '');
    }
    const wholeSetAsABundle = JSON.stringify(paths);
    const measured = svgPathBytesIn(wholeSetAsABundle);
    const budget = iconPathByteBudget(subsetPathBytes);

    expect(measured, describeBudget(measured, budget)).toBeGreaterThan(budget);
    // Not marginally: by orders of magnitude. A budget that only just rejected the whole set would
    // be one Storybook upgrade away from accepting it.
    expect(measured).toBeGreaterThan(budget * 20);
  });

  it('finds no icon id where there is none, so the grep is not matching everything', () => {
    expect(iconIdsIn('const x = 1; // no icons here at all')).toEqual(new Set());
    expect(svgPathBytesIn('a sentence of ordinary English prose, quoted: "hello world"')).toBe(0);
  });
});
