import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { expect, test } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import {
  describeBudget,
  iconIdsIn,
  iconPathByteBudget,
  svgPathBytesIn,
} from '../../dev/icon-budget.ts';
import { subsetPathBytes } from '../../src/icons/generated.ts';
import { iconRequests, requestedIconIds, iconSizes } from '../../src/icons/manifest.ts';

/**
 * The icon subset, asserted against the thing that actually ships.
 *
 * MJXOFF-181's *"Done when"*: **"Icon subsetting is asserted by bundle size — the built bundle
 * contains only referenced icons, and a test fails if the count exceeds the reference count."**
 *
 * Two measurements, answering different questions, and the pair is the point:
 *
 * * **Which icons** — the set of `mjx-fluent:` ids in `storybook-static/assets/` must **equal** the
 *   set `src/icons/manifest.ts` asks for. Equality, not a ceiling: a subset that silently *shrank*
 *   means a component is referencing an icon that is not there, which renders as a hole in a
 *   toolbar and would pass any "no more than N" assertion.
 * * **How many bytes** — the total SVG path data in the built assets, against a budget derived
 *   from the subset's own size. Id equality alone would not notice twelve megabytes of Fluent
 *   arriving by a route that never touches our ids; this would.
 *
 * `tests/icon-subset.test.ts` completes the pair by measuring the **real, whole** vendor set and
 * watching the budget refuse it — which is the ticket's *"prove it can fail by importing the whole
 * set"*, done without leaving the catalogue broken.
 *
 * ⚠ This suite reads the **built** assets. `test:browser` serves `storybook-static/` and builds
 * nothing, so running it against a stale build would test a bundle nobody produced from the current
 * source. `npm run check` chains `build-storybook` before it for exactly that reason.
 */

const assetsRoot = resolve(import.meta.dirname, '../../storybook-static/assets');

/** Every built JavaScript asset, concatenated. */
function builtJavaScript(): string {
  let names: string[];
  try {
    names = readdirSync(assetsRoot).filter((name) => name.endsWith('.js'));
  } catch {
    throw new Error(
      'storybook-static/assets is missing. Run `npm run build-storybook` first — this suite ' +
        'deliberately measures the built catalogue, not the dev server.',
    );
  }
  expect(names.length, 'the build produced no JavaScript assets').toBeGreaterThan(0);
  return names.map((name) => readFileSync(resolve(assetsRoot, name), 'utf8')).join('\n');
}

test.describe('the icon subset, in the bundle', () => {
  test('contains exactly the icons the manifest asks for, and no others', () => {
    const found = iconIdsIn(builtJavaScript());
    const requested = new Set(requestedIconIds());

    const extra = [...found].filter((id) => !requested.has(id)).sort();
    const missing = [...requested].filter((id) => !found.has(id)).sort();

    expect(
      extra,
      'the bundle carries icons src/icons/manifest.ts did not ask for. Either something imported ' +
        'the vendor package directly — which ESLint should have refused — or generated.ts was ' +
        'hand-edited, which `npm run icons:check` should have refused.',
    ).toEqual([]);
    expect(
      missing,
      'the bundle is missing icons the manifest asks for. A component referencing one of these ' +
        'renders a hole. Run `npm run icons:subset`.',
    ).toEqual([]);
  });

  test('holds the total SVG path data under a budget derived from the subset', () => {
    const measured = svgPathBytesIn(builtJavaScript());
    const budget = iconPathByteBudget(subsetPathBytes);

    // The floor matters as much as the ceiling. If the measurement ever returned zero — a
    // minifier changing its quote style, say, which has already happened once in this child — the
    // ceiling assertion would pass for ever while measuring nothing. So the subset's own bytes
    // must be *found* before the budget means anything.
    expect(
      measured,
      'no SVG path data was found in the built assets at all, which cannot be right: the subset ' +
        'alone is ' +
        `${String(subsetPathBytes)} bytes. svgPathBytesIn has stopped matching what the bundler ` +
        'emits, and the budget below is measuring nothing.',
    ).toBeGreaterThanOrEqual(subsetPathBytes);

    expect(measured, describeBudget(measured, budget)).toBeLessThanOrEqual(budget);
  });

  test('ships no vendor SVG file into the static output', () => {
    // The other route in: `staticDirs` copies `public/` verbatim, so a well-meaning contributor
    // dropping the icon directory there would put 12.7 MB into the build without any import for
    // ESLint to refuse.
    const staticRoot = resolve(import.meta.dirname, '../../storybook-static');
    const vendorNames: string[] = [];
    const walk = (directory: string): void => {
      for (const entry of readdirSync(directory, { withFileTypes: true })) {
        const path = resolve(directory, entry.name);
        if (entry.isDirectory()) walk(path);
        else if (/_\d+_(?:regular|filled|light|color)\.svg$/.test(entry.name)) {
          vendorNames.push(path);
        }
      }
    };
    walk(staticRoot);
    expect(vendorNames, 'vendor icon files were copied into the build').toEqual([]);
  });
});

test.describe('the icon element', () => {
  test('renders every glyph in the subset, at the box its size claims', async ({ page }) => {
    const story = builtStories().find(
      (entry) => entry.title === 'Foundations/Icons' && entry.name === 'The Whole Subset',
    );
    expect(story, 'Foundations/Icons · The Whole Subset is missing from the catalogue').toBeDefined();
    if (story === undefined) return;

    await openStory(page, story.id);

    const icons = page.locator('mjx-icon');
    const expected = requestedIconIds().length;
    await expect(icons).toHaveCount(expected);

    // Every one resolved. A story that rendered sixty-eight empty boxes would look plausible in a
    // screenshot and would be caught here.
    const states = await icons.evaluateAll((elements) =>
      elements.map((element) => (element as HTMLElement).dataset['iconState'] ?? '(none)'),
    );
    expect(new Set(states)).toEqual(new Set(['resolved']));

    // …and each is drawn in the box its own `size` attribute claims, which is the assertion that
    // notices a 20px drawing scaled to 24 rather than the 24px drawing being used.
    const wrong = await icons.evaluateAll((elements) =>
      elements
        .map((element) => {
          const size = Number(element.getAttribute('size'));
          const box = element.getBoundingClientRect();
          const svg = element.shadowRoot?.querySelector('svg');
          return {
            name: `${String(element.getAttribute('name'))}-${String(size)}`,
            ok:
              Math.round(box.width) === size &&
              Math.round(box.height) === size &&
              svg?.getAttribute('viewBox') === `0 0 ${String(size)} ${String(size)}`,
          };
        })
        .filter((entry) => !entry.ok)
        .map((entry) => entry.name),
    );
    expect(wrong, 'these icons are not drawn at the size they claim').toEqual([]);
  });

  test('renders the size ladder at five distinct sizes', async ({ page }) => {
    // The identity-value trap, in its iconographic form: a ladder story that rendered five copies
    // of one size would look like a ladder. So the sizes are read back and required to be five
    // different numbers, matching the five the manifest declares.
    const story = builtStories().find(
      (entry) => entry.title === 'Foundations/Icons' && entry.name === 'The Size Ladder',
    );
    expect(story).toBeDefined();
    if (story === undefined) return;

    await openStory(page, story.id);
    const widths = await page
      .locator('mjx-icon')
      .evaluateAll((elements) => elements.map((element) => Math.round(element.getBoundingClientRect().width)));
    expect(widths).toEqual([...iconSizes]);
  });

  test('refuses an icon outside the subset, and says so in the DOM', async ({ page }) => {
    const story = builtStories().find(
      (entry) => entry.title === 'Foundations/Icons' && entry.name === 'An Icon Outside The Subset',
    );
    expect(story).toBeDefined();
    if (story === undefined) return;

    // The element reports the miss on the console rather than throwing. Collecting it here is what
    // makes "it told somebody" part of the contract instead of a hope.
    const complaints: string[] = [];
    page.on('console', (message) => {
      if (message.type() === 'error') complaints.push(message.text());
    });

    await openStory(page, story.id);

    for (const probe of ['unknown-name', 'unknown-size']) {
      const element = page.locator(`mjx-icon[data-probe="${probe}"]`);
      await expect(element).toHaveAttribute('data-icon-state', 'unknown');
      // Nothing drawn — not a fallback icon. A wrong icon is worse than a missing one.
      const drawn = await element.evaluate(
        (node) => node.shadowRoot?.querySelector('svg') !== null && node.shadowRoot?.querySelector('svg') !== undefined,
      );
      expect(drawn, `${probe} drew something; it must draw nothing`).toBe(false);
      await expect(element).toHaveAttribute('aria-hidden', 'true');
    }

    expect(
      complaints.filter((text) => text.includes('mjx-fluent:')).length,
      'the element rendered nothing and told nobody',
    ).toBeGreaterThanOrEqual(2);
  });

  test('is decorative without a label and an image with one', async ({ page }) => {
    const story = builtStories().find(
      (entry) => entry.title === 'Foundations/Icons' && entry.name === 'Tinted From Tokens',
    );
    expect(story).toBeDefined();
    if (story === undefined) return;

    await openStory(page, story.id);
    // Tinted From Tokens renders unlabelled icons: they must be hidden from the accessibility tree
    // entirely, because each already sits beside its own caption.
    const decorative = page.locator('mjx-icon').first();
    await expect(decorative).toHaveAttribute('aria-hidden', 'true');
    await expect(decorative).not.toHaveAttribute('role', 'img');

    const labelled = builtStories().find(
      (entry) => entry.title === 'Foundations/Icons' && entry.name === 'The Size Ladder',
    );
    if (labelled === undefined) return;
    await openStory(page, labelled.id);
    const first = page.locator('mjx-icon').first();
    await expect(first).toHaveAttribute('role', 'img');
    await expect(first).toHaveAttribute('aria-label', /.+/);
  });

  test('takes its colour from the text around it', async ({ page }) => {
    // The one decision that makes an icon inside a pressed button turn the pressed colour with no
    // icon-specific rule anywhere: the paths are filled with `currentColor`.
    const story = builtStories().find(
      (entry) => entry.title === 'Foundations/Icons' && entry.name === 'Tinted From Tokens',
    );
    expect(story).toBeDefined();
    if (story === undefined) return;

    await openStory(page, story.id);
    const fills = await page.locator('mjx-icon').evaluateAll((elements) =>
      elements.map((element) => {
        const path = element.shadowRoot?.querySelector('path');
        const svg = element.shadowRoot?.querySelector('svg');
        return {
          fill: path === null || path === undefined ? '(none)' : getComputedStyle(path).fill,
          host: getComputedStyle(element).color,
          svgFill: svg === null || svg === undefined ? '(none)' : getComputedStyle(svg).fill,
        };
      }),
    );
    expect(fills.length).toBeGreaterThan(1);
    for (const entry of fills) expect(entry.fill).toBe(entry.host);
    // Four different tints, so the assertion above is not four copies of one colour agreeing with
    // itself — the identity-value trap again.
    expect(new Set(fills.map((entry) => entry.host)).size).toBe(fills.length);
  });

  test('the manifest, the catalogue and the bundle all agree on the count', () => {
    const total = iconRequests.reduce(
      (sum, request) => sum + request.sizes.length * request.variants.length,
      0,
    );
    expect(total).toBe(requestedIconIds().length);
    expect(iconIdsIn(builtJavaScript()).size).toBe(total);
  });
});
