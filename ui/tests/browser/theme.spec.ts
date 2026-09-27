import { expect, test } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { tokens } from '../../tokens/tokens.ts';

/**
 * Theme switching, asserted on computed values.
 *
 * `DESIGN_TOKENS.md` §2.3: *"The page stays true white (`#ffffff`) in both themes"* while the
 * canvas backdrop behind it changes. That is the defining property of the document-surface palette
 * and the classic mistake it exists to prevent — an editor that tints the page has changed what
 * the author sees relative to what they will print.
 *
 * **No hex literal appears in this file.** Every expectation is derived from the generated token
 * table, so the assertions survive the palette re-seed that is coming and would still catch a
 * theme layer that started tinting the page. What is asserted is the *contract*: the page is the
 * same colour in both schemes, the backdrop is not, and the computed value matches what the
 * generator said it would be.
 */

const surfaceStory = builtStories().find(
  (story) => story.title === 'Foundations/Document surface' && story.name === 'Page On Backdrop',
);

/** `#rrggbb` to the `rgb(r, g, b)` spelling `getComputedStyle` returns. */
function asRgb(hex: string): string {
  const value = Number.parseInt(hex.slice(1, 7), 16);
  return `rgb(${String((value >> 16) & 0xff)}, ${String((value >> 8) & 0xff)}, ${String(value & 0xff)})`;
}

test.describe('the document surface', () => {
  test('is in the catalogue at all', () => {
    expect(surfaceStory, 'Foundations/Document surface · Page On Backdrop is missing').toBeDefined();
  });

  for (const scheme of ['light', 'dark'] as const) {
    test(`resolves to the generated ${scheme} values`, async ({ page }) => {
      test.skip(surfaceStory === undefined, 'the document-surface story is missing');
      if (surfaceStory === undefined) return;
      await openStory(page, surfaceStory.id, { theme: scheme });

      const backdrop = page.locator('[data-part="backdrop"]');
      const sheet = page.locator('[data-part="page"]');

      await expect(backdrop).toHaveCSS('background-color', asRgb(tokens.document[scheme].backdrop));
      await expect(sheet).toHaveCSS('background-color', asRgb(tokens.document[scheme].page));
      await expect(sheet).toHaveCSS('border-top-color', asRgb(tokens.document[scheme].pageBorder));
    });
  }

  test('keeps the page identical while the backdrop changes', async ({ page }) => {
    test.skip(surfaceStory === undefined, 'the document-surface story is missing');
    if (surfaceStory === undefined) return;

    const read = async (theme: 'light' | 'dark') => {
      await openStory(page, surfaceStory.id, { theme });
      return page.evaluate(() => {
        const backdrop = document.querySelector('[data-part="backdrop"]');
        const sheet = document.querySelector('[data-part="page"]');
        if (backdrop === null || sheet === null) throw new Error('the probe did not render');
        return {
          backdrop: getComputedStyle(backdrop).backgroundColor,
          page: getComputedStyle(sheet).backgroundColor,
          shadow: getComputedStyle(sheet).boxShadow,
        };
      });
    };

    const light = await read('light');
    const dark = await read('dark');

    // The whole of §2.3 in three assertions.
    expect(light.page, 'the page is not a themed surface: it is paper').toBe(dark.page);
    expect(light.backdrop, 'the backdrop is what carries the theme').not.toBe(dark.backdrop);
    expect(light.shadow, 'the page shadow deepens with the backdrop').not.toBe(dark.shadow);

    // And the contract stated by the generator, so a theme that changed the page would fail here
    // even if it changed it identically in both schemes.
    expect(tokens.document.light.page).toBe(tokens.document.dark.page);
    expect(light.page).toBe(asRgb(tokens.document.light.page));
  });

  test('the system scheme is reachable, and follows prefers-color-scheme', async ({ page }) => {
    test.skip(surfaceStory === undefined, 'the document-surface story is missing');
    if (surfaceStory === undefined) return;

    // `tokens.css` has three rules and a theme toolbar that only ever writes `data-theme` would
    // leave the middle one — the system preference — unreachable and untested.
    await page.emulateMedia({ colorScheme: 'dark' });
    await openStory(page, surfaceStory.id, { theme: 'system' });
    await expect(page.locator('html')).not.toHaveAttribute('data-theme', /.*/);
    await expect(page.locator('[data-part="backdrop"]')).toHaveCSS(
      'background-color',
      asRgb(tokens.document.dark.backdrop),
    );

    await page.emulateMedia({ colorScheme: 'light' });
    await expect(page.locator('[data-part="backdrop"]')).toHaveCSS(
      'background-color',
      asRgb(tokens.document.light.backdrop),
    );
  });
});
