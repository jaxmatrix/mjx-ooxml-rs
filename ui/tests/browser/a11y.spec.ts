import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

import {
  builtStories,
  conventionsAttribute,
  disabledAxeRules,
  expectationAttribute,
  openStory,
} from './support.ts';
import { bodyTextMinimum, contrastExemplars, formatRatio } from '../../dev/contrast.ts';

/**
 * The a11y gate.
 *
 * MJXOFF-180 calls this *"the most valuable thing in the child"*, and the reason is worth writing
 * where the code is rather than in a ticket: **a sweep that only asserts that stories pass is
 * satisfied by an accessibility checker with its rules switched off.** Zero violations is what a
 * clean catalogue looks like and it is also what a disabled rule looks like, and nothing in a green
 * run distinguishes them.
 *
 * So this suite asserts in both directions:
 *
 * * every story that does not declare otherwise has **no** violations, and
 * * every story that declares `expectViolations` **does** violate, on the rule it named.
 *
 * The second half is the one that cannot be satisfied by a switched-off rule, and it is why the
 * throwaway probes in `dev/` exist at all. Fifteen further Phase U children inherit this sweep
 * automatically, because the story list comes from the build's own index rather than from a list
 * in this file.
 */

const stories = builtStories();

test.describe('the accessibility sweep', () => {
  test('the catalogue has stories, and the sweep is looking at all of them', () => {
    expect(stories.length).toBeGreaterThan(0);
    // A sweep that silently found nothing to sweep is the failure mode this line exists for.
    expect(stories.some((story) => story.title.startsWith('Gates/'))).toBe(true);
  });

  for (const story of stories) {
    test(`${story.title} · ${story.name}`, async ({ page }) => {
      await openStory(page, story.id);

      const root = page.locator('html');
      const conventions = await root.getAttribute(conventionsAttribute);
      expect(
        conventions,
        `${story.title} published no story conventions. Every story must go through ` +
          'defineStoryMeta, which is also what the mjx/story-conventions lint rule enforces.',
      ).not.toBeNull();

      const declared = await root.getAttribute(expectationAttribute);
      const expectedRules = declared === null ? [] : declared.split(' ').filter((rule) => rule !== '');

      let builder = new AxeBuilder({ page }).include('#storybook-root');
      for (const rule of Object.keys(disabledAxeRules)) builder = builder.disableRules(rule);
      const results = await builder.analyze();
      const found = results.violations.map((violation) => violation.id);

      if (expectedRules.length === 0) {
        expect(
          found,
          `${story.title} · ${story.name} has accessibility violations:\n` +
            results.violations
              .map(
                (violation) =>
                  `  ${violation.id}: ${violation.help}\n` +
                  violation.nodes
                    .map((node) => `    ${node.target.join(' ')} — ${node.failureSummary ?? ''}`)
                    .join('\n'),
              )
              .join('\n'),
        ).toEqual([]);
        return;
      }

      // The half that proves the checker is switched on.
      for (const rule of expectedRules) {
        expect(
          found,
          `${story.title} · ${story.name} declares that it must violate '${rule}', and it did ` +
            'not. Either the story stopped being broken — in which case the gate no longer ' +
            'demonstrates anything and the declaration must go — or the rule has been disabled, ' +
            'which is the failure this assertion exists to catch.',
        ).toContain(rule);
      }
    });
  }
});

test.describe('the contrast rule, as DESIGN_TOKENS.md §2.2 states it', () => {
  test('axe and this project agree about which token is legal as body text', async ({ page }) => {
    // `dev/contrast.ts` predicts the verdict from WCAG's own arithmetic; axe delivers it. Asserting
    // that the two agree is what separates "the palette is clean" from "the rule is off" — the
    // same reason mjx-paint refuses to compare a painter with itself.
    const exemplars = contrastExemplars();

    const rejected = stories.find((story) => story.name === 'Token Rejected As Body Text');
    const accepted = stories.find((story) => story.name === 'Token Accepted As Body Text');
    expect(rejected, 'the rejecting exemplar story is missing from the catalogue').toBeDefined();
    expect(accepted, 'the accepting exemplar story is missing from the catalogue').toBeDefined();
    if (rejected === undefined || accepted === undefined) return;

    expect(exemplars.rejected.ratio).toBeLessThan(bodyTextMinimum);
    expect(exemplars.accepted.ratio).toBeGreaterThanOrEqual(bodyTextMinimum);

    await openStory(page, rejected.id);
    const rejectedResults = await new AxeBuilder({ page })
      .include('#storybook-root')
      .withRules(['color-contrast'])
      .analyze();
    expect(
      rejectedResults.violations.map((violation) => violation.id),
      `${exemplars.rejected.path} measures ${formatRatio(exemplars.rejected.ratio)} on ` +
        `${exemplars.background} and must be rejected as body text.`,
    ).toContain('color-contrast');

    await openStory(page, accepted.id);
    const acceptedResults = await new AxeBuilder({ page })
      .include('#storybook-root')
      .withRules(['color-contrast'])
      .analyze();
    expect(
      acceptedResults.violations.map((violation) => violation.id),
      `${exemplars.accepted.path} measures ${formatRatio(exemplars.accepted.ratio)} on ` +
        `${exemplars.background} and must be accepted as body text.`,
    ).toEqual([]);
  });
});
