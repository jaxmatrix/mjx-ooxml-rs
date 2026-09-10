import { expect, test } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { customProperties } from '../../tokens/tokens.ts';
// The data module, not `dev/probes.ts`: a spec runs in Node and cannot import a module that
// evaluates `class extends HTMLElement`.
import { firstColorToken, generatedValue } from '../../dev/token-choice.ts';

/**
 * The resolution order, end to end.
 *
 * `DESIGN_TOKENS.md` §3: *explicit host configuration → CSS custom properties read off the host
 * element → built-in defaults*. Three legs, and this suite exercises each of them **and the
 * boundaries between them**, because a resolver that returns the right value for the wrong reason
 * passes an equality assertion and fails a host.
 *
 * The override value is a magenta sentinel rather than another token, on purpose: a second token
 * could coincide with the default and the test would pass without anything having been adopted.
 *
 * No token name is written here either. The probe reads whichever token the generated table lists
 * first, so the suite survives the palette re-seed.
 */

const token = firstColorToken();
const property = customProperties[token];
const sentinel = '#ff00ff';
const sentinelRgb = 'rgb(255, 0, 255)';

const story = builtStories().find(
  (entry) => entry.title === 'Gates/Token resolution' && entry.name === 'Generated Default',
);

/** The generated default for the probed token, as a value rather than a name. */
const generatedDefault = (): string => generatedValue(token);

test.describe('the token resolver', () => {
  test('is in the catalogue at all', () => {
    expect(story, 'Gates/Token resolution · Generated Default is missing').toBeDefined();
    expect(property, `the generated table has no custom property for ${token}`).toBeDefined();
  });

  test('adopts a custom property set on the host, and returns to the default when it goes', async ({
    page,
  }) => {
    test.skip(story === undefined, 'the token-resolution story is missing');
    if (story === undefined || property === undefined) return;
    await openStory(page, story.id);

    const probe = page.locator('mjx-token-probe').first();

    // Leg 3 — nothing set anywhere. `tokens.css` is loaded at :root, so the *host* leg answers
    // here; what matters is that the value is the generated one.
    await expect(probe).toHaveAttribute('data-resolved', generatedDefault());

    // Leg 2 — the host declares the property. This is the brief's *"if tokens are set, adopt
    // them"*, and it is the leg that lets a host re-theme the platform with no code change.
    await page.evaluate(
      ([name, value]) => {
        document.querySelector('mjx-token-probe')?.style.setProperty(name ?? '', value ?? '');
      },
      [property, sentinel],
    );
    await expect(probe).toHaveAttribute('data-resolved', sentinel);
    await expect(probe).toHaveAttribute('data-origin', 'host');
    await expect(probe.locator('span').first()).toHaveCSS('background-color', sentinelRgb);

    // And away again. A resolver that cached would pass everything above and fail this line.
    await page.evaluate((name) => {
      document.querySelector('mjx-token-probe')?.style.removeProperty(name);
    }, property);
    await expect(probe).toHaveAttribute('data-resolved', generatedDefault());
  });

  test('the MutationObserver is what noticed, not a re-render', async ({ page }) => {
    test.skip(story === undefined, 'the token-resolution story is missing');
    if (story === undefined || property === undefined) return;
    await openStory(page, story.id);

    // Nothing below calls `refresh()`. If the resolver were not watching, the attribute would stay
    // at the default and this test would fail — which is the only way to tell "it recomputed" from
    // "it happened to be re-rendered by something else".
    await page.evaluate(
      ([name, value]) => {
        document.querySelector('mjx-token-probe')?.setAttribute('style', `${name ?? ''}: ${value ?? ''}`);
      },
      [property, sentinel],
    );
    await expect(page.locator('mjx-token-probe').first()).toHaveAttribute('data-resolved', sentinel);
  });

  test('reports the generated default when the cascade has nothing to say', async ({ page }) => {
    test.skip(story === undefined, 'the token-resolution story is missing');
    if (story === undefined) return;
    await openStory(page, story.id);

    // The third leg is not a duplicate of tokens.css — it is what answers when the stylesheet is
    // not loaded at all, which is the ordinary case for a platform embedded in someone else's
    // application. A detached element sees no cascade, which is the closest a loaded page can get
    // to that condition.
    const [resolved, origin] = await page.evaluate((path) => {
      const probe = document.createElement('mjx-token-probe');
      probe.setAttribute('token', path);
      // Deliberately NOT appended to the document.
      const constructor = customElements.get('mjx-token-probe');
      if (constructor === undefined) throw new Error('the probe is not registered');
      probe.connectedCallback();
      probe.refresh();
      return [probe.dataset['resolved'], probe.dataset['origin']];
    }, token);

    expect(resolved).toBe(generatedDefault());
    expect(origin).toBe('default');
  });

  test('explicit configuration beats a host property that is also set', async ({ page }) => {
    test.skip(story === undefined, 'the token-resolution story is missing');
    if (story === undefined || property === undefined) return;
    await openStory(page, story.id);

    const explicit = '#00ffff';
    const probe = page.locator('mjx-token-probe').first();

    // Both lower legs are made to answer, and then the top one is added over them. The order is
    // only proved by setting *both*: with the host property absent, an explicit value that won
    // would be indistinguishable from a default that happened to match.
    await page.evaluate(
      ([name, value]) => {
        document.querySelector('mjx-token-probe')?.style.setProperty(name ?? '', value ?? '');
      },
      [property, sentinel],
    );
    await expect(probe).toHaveAttribute('data-origin', 'host');

    await page.evaluate((value) => {
      document.querySelector('mjx-token-probe')?.setAttribute('explicit', value);
    }, explicit);
    await expect(probe).toHaveAttribute('data-resolved', explicit);
    await expect(probe).toHaveAttribute('data-origin', 'explicit');

    // And removing it falls back to the host property, not to the default: the legs are ordered,
    // not exclusive.
    await page.evaluate(() => {
      document.querySelector('mjx-token-probe')?.removeAttribute('explicit');
    });
    await expect(probe).toHaveAttribute('data-resolved', sentinel);
    await expect(probe).toHaveAttribute('data-origin', 'host');
  });
});
