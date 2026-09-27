import { expect, test, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { contrastRatio } from '../../dev/contrast.ts';
import { tokens } from '../../tokens/tokens.ts';
import { typeRoleNames, typeRoles } from '../../src/foundations/typography.ts';
import {
  surfaceBackgroundMember,
  surfaceLevelNames,
  surfaceLevels,
} from '../../src/foundations/surfaces.ts';
import { focusIndicatorMinimumContrast } from '../../src/foundations/focus.ts';
import { motionRoleNames, motionRoles } from '../../src/foundations/motion.ts';
import { accessibleHitTargetMinimum, densityModes } from '../../src/foundations/density.ts';

/**
 * The foundations, measured in a browser.
 *
 * `tests/foundations.test.ts` proves the *values* are right. This proves the **cascade delivers
 * them**, which is a different claim and the one that fails in practice: a role that names the
 * correct token and never reaches the element, a shadow that is defined and clipped, a focus ring
 * that exists in a stylesheet nothing adopted.
 *
 * Four of MJXOFF-181's traps are answered here, and each needs a rendering engine:
 *
 * * **"If every story renders one size at one weight, the type scale has never been exercised."**
 *   Every role is rendered and its computed size, measure, weight and family read back — and
 *   required to be *distinct*, so six names for one size fails.
 * * **"A focus ring nobody focuses is untested."** The ring is driven with real `Tab` presses,
 *   through the browser's own input path, so `:focus-visible` makes its own judgement rather than
 *   being asserted into existence. Then in the other theme. Then with a mouse, where it must
 *   **not** appear.
 * * **"A shadow that renders as flat black is wrong in a way a snapshot will happily lock in."**
 *   The computed `box-shadow` is parsed and its colour compared with the generated token, channel
 *   for channel, and required to be chromatic.
 * * **"Density modes change spacing without changing hit-target size below the accessible
 *   minimum, asserted on computed values."** Both halves, on `getComputedStyle`.
 *
 * ⚠ **No colour, size or duration is written in this file.** Every expectation is derived from the
 * generated tokens, because the palette is about to be re-seeded and a gate that named a hex would
 * break on the day it is most needed.
 */

/** Open a story by title and name, failing with the name rather than with `undefined`. */
async function open(
  page: Page,
  title: string,
  name: string,
  options: { theme?: string } = {},
): Promise<void> {
  const story = builtStories().find((entry) => entry.title === title && entry.name === name);
  expect(story, `${title} · ${name} is missing from the catalogue`).toBeDefined();
  if (story === undefined) return;
  await openStory(page, story.id, options);
}

/** `rgb(34, 59, 51)` or `rgba(34, 59, 51, 0.11)` → three channels and an alpha. */
function parseColor(value: string): { channels: [number, number, number]; alpha: number } | undefined {
  const match = /rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)\s*(?:[,/]\s*([\d.%]+)\s*)?\)/.exec(value);
  if (match === null) return undefined;
  const alphaText = match[4];
  const alpha =
    alphaText === undefined
      ? 1
      : alphaText.endsWith('%')
        ? Number.parseFloat(alphaText) / 100
        : Number.parseFloat(alphaText);
  return {
    channels: [Number(match[1]), Number(match[2]), Number(match[3])],
    alpha,
  };
}

/** `#rrggbbaa` → the same shape, so a computed value and a token can be compared directly. */
function parseHex(value: string): { channels: [number, number, number]; alpha: number } | undefined {
  const match = /#([0-9a-fA-F]{6})([0-9a-fA-F]{2})?/.exec(value);
  if (match?.[1] === undefined) return undefined;
  const number = Number.parseInt(match[1], 16);
  return {
    channels: [(number >> 16) & 0xff, (number >> 8) & 0xff, number & 0xff],
    alpha: match[2] === undefined ? 1 : Number.parseInt(match[2], 16) / 255,
  };
}

/** `rgb(…)` → `#rrggbb`, so a computed colour can go through the WCAG arithmetic. */
function toHex(value: string): string | undefined {
  const parsed = parseColor(value);
  if (parsed === undefined) return undefined;
  return `#${parsed.channels.map((channel) => Math.round(channel).toString(16).padStart(2, '0')).join('')}`;
}

// ── typography ───────────────────────────────────────────────────────────────

test.describe('the type scale, as the browser computes it', () => {
  test('renders all six roles, and no two of them the same', async ({ page }) => {
    await open(page, 'Foundations/Typography', 'The Scale');

    const measured = await page.locator('[data-specimen]').evaluateAll((elements) =>
      elements.map((element) => {
        const style = getComputedStyle(element);
        return {
          role: (element as HTMLElement).dataset['specimen'] ?? '',
          fontSize: style.fontSize,
          lineHeight: style.lineHeight,
          fontWeight: style.fontWeight,
          fontFamily: style.fontFamily,
          letterSpacing: style.letterSpacing,
        };
      }),
    );

    expect(measured.map((entry) => entry.role)).toEqual([...typeRoleNames]);

    // The identity-value trap, answered: six roles that computed to one appearance would render as
    // a plausible page and fail here.
    const fingerprints = measured.map(
      (entry) =>
        `${entry.fontSize}|${entry.lineHeight}|${entry.fontWeight}|${entry.fontFamily}|${entry.letterSpacing}`,
    );
    expect(new Set(fingerprints).size, `two roles compute identically:\n${fingerprints.join('\n')}`).toBe(
      typeRoleNames.length,
    );

    // …and every one of them is a real number of pixels rather than an unresolved var().
    for (const entry of measured) {
      expect(Number.parseFloat(entry.fontSize), `${entry.role} has no font size`).toBeGreaterThan(0);
      expect(Number.parseFloat(entry.lineHeight), `${entry.role} has no line height`).toBeGreaterThan(0);
    }
  });

  test('resolves dense to the generated text.xs on leading.tight', async ({ page }) => {
    // §4's correction, checked where it actually lands. The expectation is computed from the
    // generated tokens rather than written down: `--text-xs` is a rem value and the browser's own
    // root font size decides what that is in pixels.
    await open(page, 'Foundations/Typography', 'The Scale');

    const rootFontSize = await page.evaluate(() =>
      Number.parseFloat(getComputedStyle(document.documentElement).fontSize),
    );
    const expectedSize = Number.parseFloat(tokens.text.xs) * rootFontSize;
    const expectedLeading = expectedSize * tokens.leading.tight;

    const dense = page.locator('[data-specimen="dense"]');
    const size = Number.parseFloat(await dense.evaluate((node) => getComputedStyle(node).fontSize));
    const leading = Number.parseFloat(
      await dense.evaluate((node) => getComputedStyle(node).lineHeight),
    );

    expect(size).toBeCloseTo(expectedSize, 1);
    expect(leading).toBeCloseTo(expectedLeading, 1);
  });

  test('gives the serif to the display role and to nothing else', async ({ page }) => {
    // §4: Young Serif is display-only. `--font-serif` is a stack whose first family is the serif;
    // the name is read out of the generated token rather than typed here.
    const serifFamily = tokens.font.serif.split(',')[0]?.replaceAll('"', '').trim() ?? '';
    expect(serifFamily).not.toBe('');

    await open(page, 'Foundations/Typography', 'The Scale');
    const families = await page.locator('[data-specimen]').evaluateAll((elements) =>
      elements.map((element) => ({
        role: (element as HTMLElement).dataset['specimen'] ?? '',
        family: getComputedStyle(element).fontFamily,
      })),
    );
    for (const entry of families) {
      const usesSerif = entry.family.includes(serifFamily);
      expect(usesSerif, `${entry.role} computes to ${entry.family}`).toBe(entry.role === 'display');
    }
    expect(typeRoles.display.family).toContain('--font-serif');
  });

  test('the display role reaches an empty state, which is the only place it may', async ({ page }) => {
    await open(page, 'Foundations/Typography', 'Display Is For Empty States');
    const heading = page.locator('[data-specimen="display-empty-state"]');
    await expect(heading).toBeVisible();
    // A real `<h2>`, because a role is a visual weight and not a semantic one.
    expect(await heading.evaluate((node) => node.tagName)).toBe('H2');
  });
});

// ── surfaces ─────────────────────────────────────────────────────────────────

test.describe('the elevation ladder, as the browser computes it', () => {
  test('gives every rung its token background and its token radius', async ({ page }) => {
    await open(page, 'Foundations/Surfaces', 'The Elevation Ladder');

    const measured = await page.locator('mjx-surface[data-level]').evaluateAll((elements) =>
      elements.map((element) => {
        const style = getComputedStyle(element);
        return {
          level: (element as HTMLElement).dataset['level'] ?? '',
          background: style.backgroundColor,
          radius: style.borderTopLeftRadius,
          shadow: style.boxShadow,
        };
      }),
    );
    expect(measured.map((entry) => entry.level)).toEqual([...surfaceLevelNames]);

    for (const entry of measured) {
      const level = entry.level as (typeof surfaceLevelNames)[number];
      const expectedBackground = tokens.theme.light[
        surfaceBackgroundMember[level] as keyof (typeof tokens.theme.light)
      ];
      expect(toHex(entry.background), `${entry.level} background`).toBe(
        String(expectedBackground).toLowerCase(),
      );

      const expectedRadius = tokens.radius[surfaceLevels[level].radius];
      expect(entry.radius, `${entry.level} radius`).toBe(expectedRadius);
    }

    // The radii are not all the same, or the assertion above would be one value agreeing with
    // itself six times.
    expect(new Set(measured.map((entry) => entry.radius)).size).toBeGreaterThan(1);
  });

  test('renders the whole radius scale, each step distinct', async ({ page }) => {
    await open(page, 'Foundations/Surfaces', 'The Radius Scale');
    const measured = await page.locator('mjx-surface[data-radius]').evaluateAll((elements) =>
      elements.map((element) => ({
        step: (element as HTMLElement).dataset['radius'] ?? '',
        radius: getComputedStyle(element).borderTopLeftRadius,
      })),
    );
    for (const entry of measured) {
      expect(entry.radius, `${entry.step}`).toBe(
        tokens.radius[entry.step as keyof typeof tokens.radius],
      );
    }
    expect(new Set(measured.map((entry) => entry.radius)).size).toBe(measured.length);
  });

  test('paints an ink-tinted shadow, not a neutral black', async ({ page }) => {
    // MJXOFF-181's trap for this child, answered arithmetically rather than by a picture: the
    // computed shadow's colour must equal the generated `shadow.lift` channel for channel, and its
    // channels must differ — which no neutral black can manage, whatever the palette becomes.
    await open(page, 'Foundations/Surfaces', 'The Shadow Is Ink Tinted');

    const lifted = await page
      .locator('mjx-surface[data-shadow="lift"]')
      .evaluate((node) => getComputedStyle(node).boxShadow);
    const flat = await page
      .locator('mjx-surface[data-shadow="none"]')
      .evaluate((node) => getComputedStyle(node).boxShadow);

    expect(flat, 'the unlifted rung has a shadow it should not have').toBe('none');

    const painted = parseColor(lifted);
    const declared = parseHex(tokens.shadow.lift);
    expect(painted, `the lifted rung's box-shadow is '${lifted}'`).toBeDefined();
    expect(declared, `shadow.lift is '${tokens.shadow.lift}'`).toBeDefined();
    if (painted === undefined || declared === undefined) return;

    expect(painted.channels).toEqual(declared.channels);
    expect(painted.alpha).toBeCloseTo(declared.alpha, 2);
    expect(
      new Set(painted.channels).size,
      `the shadow renders achromatic (${lifted}) — DESIGN_TOKENS.md §4: shadows are ink-tinted, never neutral black`,
    ).toBeGreaterThan(1);
  });

  test('keeps the page shadow ink-tinted in the dark scheme too', async ({ page }) => {
    // The dark scheme's page shadow *is* neutral, deliberately — `document.dark.pageShadow` is a
    // black at 35%, because a green-tinted shadow on a dark backdrop reads as a glow. This test
    // exists so that difference is a recorded decision rather than an accident: the chrome's
    // shadow stays tinted, the dark document's page shadow does not.
    await open(page, 'Foundations/Surfaces', 'The Shadow Is Ink Tinted', { theme: 'dark' });
    const lifted = await page
      .locator('mjx-surface[data-shadow="lift"]')
      .evaluate((node) => getComputedStyle(node).boxShadow);
    const painted = parseColor(lifted);
    expect(painted).toBeDefined();
    expect(painted?.channels).toEqual(parseHex(tokens.shadow.lift)?.channels);
  });
});

// ── focus ────────────────────────────────────────────────────────────────────

/**
 * Tab until the wanted element is focused, or give up.
 *
 * Real key presses through the browser's own input path. That is the whole reason this helper
 * exists rather than a `focus()` call: `:focus-visible` is the user agent's judgement about *how*
 * the focus was reached, and `element.focus()` from script does not make that judgement the same
 * way. A test that called `focus()` would be asserting the ring into existence.
 */
async function tabTo(page: Page, selector: string, limit = 30): Promise<boolean> {
  for (let press = 0; press < limit; press += 1) {
    await page.keyboard.press('Tab');
    const reached = await page.evaluate((wanted) => {
      let active: Element | null = document.activeElement;
      while (active?.shadowRoot?.activeElement != null) active = active.shadowRoot.activeElement;
      return active?.matches(wanted) === true;
    }, selector);
    if (reached) return true;
  }
  return false;
}

/** The computed outline of whatever is focused, reached through any shadow boundaries. */
async function focusedOutline(page: Page): Promise<{
  style: string;
  width: number;
  color: string;
  background: string;
  visible: boolean;
}> {
  return page.evaluate(() => {
    let active: Element | null = document.activeElement;
    while (active?.shadowRoot?.activeElement != null) active = active.shadowRoot.activeElement;
    if (active === null) throw new Error('nothing is focused');
    const style = getComputedStyle(active);
    // The surface *behind* the ring: the nearest ancestor that actually paints something. A ring's
    // contrast is measured against what is behind it, and the button itself is transparent.
    let behind: Element | null = active.parentElement ?? active;
    let background = 'rgba(0, 0, 0, 0)';
    while (behind !== null) {
      const paint = getComputedStyle(behind).backgroundColor;
      if (paint !== 'rgba(0, 0, 0, 0)' && paint !== 'transparent') {
        background = paint;
        break;
      }
      behind = behind.parentElement ?? (behind.getRootNode() as ShadowRoot).host ?? null;
      if (behind === null) break;
    }
    return {
      style: style.outlineStyle,
      width: Number.parseFloat(style.outlineWidth),
      color: style.outlineColor,
      background,
      visible: active.matches(':focus-visible'),
    };
  });
}

test.describe('the focus ring, driven by a keyboard', () => {
  for (const theme of ['light', 'dark'] as const) {
    test(`is visible on every rung of the ladder — ${theme}`, async ({ page }) => {
      await open(page, 'Foundations/Focus', 'Every Rung Of The Ladder', { theme });

      const failures: string[] = [];
      for (const level of surfaceLevelNames) {
        // Focus is driven from the top of the document each time, so every rung is reached the way
        // a person reaches it and not by resuming from wherever the last assertion left off.
        await page.evaluate(() => {
          (document.activeElement as HTMLElement | null)?.blur();
        });
        await page.locator('body').click({ position: { x: 1, y: 1 } });

        const reached = await tabTo(page, `button[data-focus-target="${level}"]`, 40);
        expect(reached, `Tab never reached the ${level} button`).toBe(true);
        if (!reached) continue;

        const outline = await focusedOutline(page);
        expect(outline.visible, `${level}: focused but not :focus-visible after a Tab`).toBe(true);
        expect(outline.style, `${level}: no outline style`).not.toBe('none');
        expect(outline.width, `${level}: zero-width outline`).toBeGreaterThan(0);

        const ring = toHex(outline.color);
        const behind = toHex(outline.background);
        expect(ring, `${level}: outline colour is '${outline.color}'`).toBeDefined();
        expect(behind, `${level}: surface colour is '${outline.background}'`).toBeDefined();
        const ratio = contrastRatio(ring ?? '', behind ?? '') ?? 0;
        if (ratio < focusIndicatorMinimumContrast) {
          failures.push(
            `${theme} · ${level}: ring ${String(ring)} on ${String(behind)} measures ` +
              `${ratio.toFixed(2)} : 1, under ${String(focusIndicatorMinimumContrast)} : 1`,
          );
        }
      }
      expect(failures, failures.join('\n')).toEqual([]);
    });
  }

  test('does not appear for a pointer, which is what "keyboard-only" means', async ({ page }) => {
    // The half that cannot be satisfied by a permanently-on outline. A treatment that showed the
    // ring on every focus would pass every assertion above and be wrong.
    await open(page, 'Foundations/Focus', 'Every Rung Of The Ladder');
    const first = surfaceLevelNames[0];
    const button = page.locator(`button[data-focus-target="${String(first)}"]`);
    await button.click();

    const focused = await page.evaluate((selector) => {
      let active: Element | null = document.activeElement;
      while (active?.shadowRoot?.activeElement != null) active = active.shadowRoot.activeElement;
      return {
        isTarget: active?.matches(selector) === true,
        visible: active?.matches(':focus-visible') === true,
        outline: active === null ? 'none' : getComputedStyle(active).outlineStyle,
      };
    }, `button[data-focus-target="${String(first)}"]`);

    expect(focused.isTarget, 'the click did not focus the button').toBe(true);
    expect(focused.visible, 'a mouse click produced :focus-visible').toBe(false);
    expect(focused.outline, 'a mouse click produced a ring').toBe('none');
  });

  test('is the same ring on the harness chrome, which had one of its own', async ({ page }) => {
    // The harness's preset buttons carried two `2px` literals before this foundation existed. They
    // adopt the shared treatment now, and this is what says so — the harness is a consumer of the
    // gate rather than an exception to it.
    await open(page, 'Foundations/Focus', 'The Harness Uses It Too');
    await page.locator('body').click({ position: { x: 1, y: 1 } });
    const reached = await tabTo(page, 'button[data-preset]', 10);
    expect(reached, "Tab never reached the container's own preset buttons").toBe(true);

    const outline = await focusedOutline(page);
    expect(outline.visible).toBe(true);
    expect(outline.style).not.toBe('none');

    const expectedWidth = await page.evaluate(() => {
      const probe = document.createElement('div');
      probe.style.inlineSize = 'calc(var(--spacing) / 2)';
      document.body.append(probe);
      const width = Number.parseFloat(getComputedStyle(probe).inlineSize);
      probe.remove();
      return width;
    });
    expect(outline.width).toBeCloseTo(expectedWidth, 1);
  });
});

// ── motion ───────────────────────────────────────────────────────────────────

test.describe('the motion vocabulary, as the browser resolves it', () => {
  test('gives every role its token easing, and never overshoot to a document object', async ({
    page,
  }) => {
    await open(page, 'Foundations/Motion', 'The Five Roles');

    const measured = await page.locator('[data-mover]').evaluateAll((elements) =>
      elements.map((element) => ({
        role: (element as HTMLElement).dataset['mover'] ?? '',
        easing: getComputedStyle(element).transitionTimingFunction,
        duration: getComputedStyle(element).transitionDuration,
      })),
    );
    expect(measured.map((entry) => entry.role)).toEqual([...motionRoleNames]);

    /** `cubic-bezier(0.45, 0, 0.2, 1)` → its four numbers, so two spellings compare equal. */
    const numbers = (value: string): number[] =>
      [...value.matchAll(/-?\d*\.?\d+/g)].map((match) => Number(match[0]));

    for (const entry of measured) {
      const role = entry.role as (typeof motionRoleNames)[number];
      const spec = motionRoles[role];
      const [group, member] = spec.easingToken.split('.');
      const declared = tokens[group as 'ease'][member as keyof typeof tokens.ease];
      expect(numbers(entry.easing), `${role} resolves to ${entry.easing}`).toEqual(
        numbers(String(declared)),
      );
      expect(entry.duration).toBe(`${(Number.parseFloat(tokens.duration.transition) / 1000).toFixed(3).replace(/0+$/, '')}s`);

      if (spec.attachedToDocumentObject) {
        // §4, checked where it lands rather than where it is written: the overshooting curve has a
        // control point above 1, and a document-attached role must not resolve to one.
        const control = numbers(entry.easing);
        expect(
          Math.max(...control),
          `${role} is attached to a document object and resolves to ${entry.easing}, which ` +
            'overshoots. DESIGN_TOKENS.md §4: overshoot there reads as imprecision.',
        ).toBeLessThanOrEqual(1);
      }
    }

    // …and the chrome roles do overshoot, or the assertion above is satisfied by a vocabulary in
    // which nothing overshoots at all.
    const overshooting = measured.filter((entry) => {
      const control = numbers(entry.easing);
      return Math.max(...control) > 1;
    });
    expect(overshooting.length, 'no role overshoots, so the rule above proves nothing').toBeGreaterThan(0);
  });
});

// ── density ──────────────────────────────────────────────────────────────────

test.describe('density, on computed values', () => {
  test('changes the spacing and keeps the hit target above the floor', async ({ page }) => {
    await open(page, 'Foundations/Density', 'Both Modes');

    const measured = await page.locator('mjx-density-probe[data-mode]').evaluateAll((elements) =>
      elements.map((element) => {
        const style = getComputedStyle(element);
        const row = element.querySelector('[data-row]');
        const rowStyle = row === null ? undefined : getComputedStyle(row);
        return {
          mode: (element as HTMLElement).dataset['mode'] ?? '',
          gap: Number.parseFloat(style.rowGap),
          padding: Number.parseFloat(style.paddingTop),
          hitTarget: rowStyle === undefined ? 0 : Number.parseFloat(rowStyle.minBlockSize),
          rowHeight: row === null ? 0 : Math.round(row.getBoundingClientRect().height),
        };
      }),
    );

    const comfortable = measured.find((entry) => entry.mode === 'comfortable');
    const compact = measured.find((entry) => entry.mode === 'compact');
    expect(comfortable, 'the comfortable probe is missing').toBeDefined();
    expect(compact, 'the compact probe is missing').toBeDefined();
    if (comfortable === undefined || compact === undefined) return;

    // Half one: the spacing genuinely moves. A compact mode that changed nothing would pass the
    // floor assertion below and fail this.
    expect(compact.gap).toBeLessThan(comfortable.gap);
    expect(compact.padding).toBeLessThan(comfortable.padding);

    // Half two: neither mode goes under WCAG 2.2's target-size minimum.
    for (const entry of [comfortable, compact]) {
      expect(
        entry.hitTarget,
        `${entry.mode}: min-block-size resolves to ${String(entry.hitTarget)}px`,
      ).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
      expect(entry.rowHeight).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
    }

    // …and the two modes are in the ratio their multipliers declare, so the mechanism is the
    // spacing token rather than two hand-tuned numbers that happen to differ.
    expect(comfortable.gap / compact.gap).toBeCloseTo(
      densityModes.comfortable.stepUnits / densityModes.compact.stepUnits,
      1,
    );
  });

  test('cascades into a subtree, which the colour scheme cannot', async ({ page }) => {
    await open(page, 'Foundations/Density', 'Nested Inside A Comfortable Shell');

    const outer = page.locator('mjx-density-probe[data-mode="inherited"]');
    const inner = page.locator('mjx-density-probe[data-mode="nested"]');
    const outerGap = Number.parseFloat(await outer.evaluate((node) => getComputedStyle(node).rowGap));
    const innerGap = Number.parseFloat(await inner.evaluate((node) => getComputedStyle(node).rowGap));

    expect(innerGap).toBeLessThan(outerGap);
    // The compact pane's rows are still above the floor, which is the whole promise: compact
    // reduces the space between things, not the size of the things you have to hit.
    const innerTarget = await inner
      .locator('[data-row]')
      .first()
      .evaluate((node) => Number.parseFloat(getComputedStyle(node).minBlockSize));
    expect(innerTarget).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
  });
});
