import { expect, test, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import { accessibleHitTargetMinimum } from '../../src/foundations/density.ts';
import { cellsInWindow, galleryRowPlan } from '../../src/inputs/input-model.ts';
import { contrastRatioOrWorst, formatRatio, nonTextMinimum } from '../../src/tokens/contrast.ts';
import {
  formatColorChoice,
  nextSwatchIndex,
  parseColorChoice,
  pickerStoryTitles,
  substitutionNote,
  swatchSections,
  swatchStateOf,
  swatchStates,
  type ColorChoice,
} from '../../src/pickers/picker-model.ts';
import { placeFloating, type Rect } from '../../src/overlay/floating.ts';

/**
 * The pickers, measured and driven in a real browser.
 *
 * Three of this child's assertions can only be made here, and each of them is one a static story
 * would agree with while being wrong:
 *
 * * **the selection indicator is measured on the colours the browser actually composited**, not on
 *   the ones the model predicts. The unit sweep proves the *rule* over four thousand colours; this
 *   proves that the rule survives CSS resolution, custom-property inheritance and the scheme layer;
 * * **every row of a list of faces is one row tall**, which is U06's virtualisation precondition
 *   and is violated by construction the moment each name is drawn in its own face. There is no way
 *   to see it without a layout engine, and the symptom of getting it wrong is a list that scrolls
 *   to *nearly* the right font;
 * * **a declared non-colour cue is actually drawn.** A cue that is declared and not rendered fails
 *   louder than no declaration at all — U06's sixth defect, and U07 restated it.
 *
 * MJXOFF-182's rule decides the shape of everything about appearance: **a distinctness gate proves
 * no two states are the same; it does not prove any of them is right.** So the swatch suite asserts
 * correspondence — a measured colour equals what the model says — and distinctness separately.
 */

const colour = pickerStoryTitles.colorPicker;
const font = pickerStoryTitles.fontPicker;

const themeStory = { title: colour, name: 'A Document’s Own Theme' } as const;
const holesStory = { title: colour, name: 'A Theme With Holes' } as const;
const noThemeStory = { title: colour, name: 'No Theme Supplied' } as const;
const offGridStory = { title: colour, name: 'A Colour That Is On No Grid' } as const;
const flipStory = { title: colour, name: 'At The Bottom Of Its Room' } as const;
const rtlStory = { title: colour, name: 'Under Right To Left' } as const;
const compactColourStory = { title: colour, name: 'In Compact Density' } as const;

const warningStory = { title: font, name: 'What The Warning Is For' } as const;
const rowsStory = { title: font, name: 'Every Row Says What It Is' } as const;
const facesStory = { title: font, name: 'Four Hundred Rows Of Faces' } as const;
const fontStatesStory = { title: font, name: 'The States Matrix' } as const;
const compactFontStory = { title: font, name: 'In Compact Density' } as const;

const schemes: readonly ColorScheme[] = ['light', 'dark'];
const transitionMilliseconds = Number.parseFloat(tokens.duration.transition);

async function open(
  page: Page,
  story: { readonly title: string; readonly name: string },
  options: { theme?: ColorScheme; containerPreset?: string } = {},
): Promise<void> {
  const entry = builtStories().find(
    (candidate) => candidate.title === story.title && candidate.name === story.name,
  );
  expect(entry, `${story.title} · ${story.name} is missing from the catalogue`).toBeDefined();
  if (entry === undefined) return;
  await openStory(page, entry.id, options);
  await page.waitForFunction(() => customElements.get('mjx-color-picker') !== undefined);
  await page.waitForFunction(() => customElements.get('mjx-font-picker') !== undefined);
  await settle(page);
}

async function settle(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
}

/** Two frames is a layout change; a paint change needs the transition the token declares. */
async function settlePaint(page: Page): Promise<void> {
  await settle(page);
  await page.waitForTimeout(transitionMilliseconds * 2 + 120);
}

/** `#2e9e63` → `rgb(46, 158, 99)`; `transparent` → what a browser reports for it. */
function expectedColor(member: string | undefined, scheme: ColorScheme): string {
  if (member === undefined || member === 'transparent') return 'rgba(0, 0, 0, 0)';
  const hex = tokens.theme[scheme][member as keyof (typeof tokens.theme)['light']];
  const number = Number.parseInt(hex.slice(1, 7), 16);
  return `rgb(${String((number >> 16) & 0xff)}, ${String((number >> 8) & 0xff)}, ${String(number & 0xff)})`;
}

/**
 * `rgb(18, 52, 87)` → `#123457`, or `undefined` when a browser reported something else.
 *
 * ⚠ **`undefined` and never a stand-in colour.** A helper that answered black for a value it could
 * not read would make every contrast assertion below pass on a page where nothing was painted —
 * which is U07's second defect, and the reason `contrastRatioOrWorst` exists in the first place.
 */
function hexOf(computed: string): string | undefined {
  const match = /^rgba?\(\s*(\d+)[,\s]+(\d+)[,\s]+(\d+)/.exec(computed.trim());
  if (match === null) return undefined;
  const parts = [match[1], match[2], match[3]].map((part) => Number.parseInt(part ?? '', 10));
  if (parts.some((part) => Number.isNaN(part))) return undefined;
  return `#${parts.map((part) => part.toString(16).padStart(2, '0')).join('')}`;
}

/** The active element, followed through every shadow root it is hiding in. */
async function deepActive(page: Page): Promise<{ tag: string; role: string; name: string }> {
  return page.evaluate(() => {
    let element: Element | null = document.activeElement;
    while (element?.shadowRoot?.activeElement != null) element = element.shadowRoot.activeElement;
    return {
      tag: element?.localName ?? '',
      role: element?.getAttribute('role') ?? '',
      name: element?.getAttribute('aria-label') ?? '',
    };
  });
}

/** What a fixed number of `Tab` presses lands on, including the empty landings. */
async function tabStops(page: Page, presses = 8): Promise<string[]> {
  const seen: string[] = [];
  for (let index = 0; index < presses; index += 1) {
    await page.keyboard.press('Tab');
    const active = await deepActive(page);
    seen.push(`${active.tag}[${active.role}]${active.name === '' ? '' : `:${active.name}`}`);
  }
  return seen;
}

// ── the swatch's states, on live cells ───────────────────────────────────────

test.describe('a swatch', () => {
  for (const scheme of schemes) {
    test(`every state computes what the model says, in ${scheme}`, async ({ page }) => {
      await open(page, holesStory, { theme: scheme });
      await settlePaint(page);

      const cells = page.locator('#partial-theme [role="option"]');
      // 1 reset chip + 60 theme + 10 standard.
      await expect(cells).toHaveCount(71);

      const facts = await cells.evaluateAll((elements) =>
        elements.map((element) => ({
          selected: element.getAttribute('aria-selected') === 'true',
          active: element.hasAttribute('data-active'),
          unavailable: element.getAttribute('aria-disabled') === 'true',
          borderColor: getComputedStyle(element).borderTopColor,
          borderStyle: getComputedStyle(element).borderTopStyle,
          shadow: getComputedStyle(element).boxShadow,
        })),
      );

      const produced = facts.map((cell) => swatchStateOf(cell));
      // ⚠ Anti-vacuity. A story where every cell was resting would satisfy every per-cell
      // assertion below and prove nothing at all.
      expect(new Set(produced).size, `only produced ${[...new Set(produced)].join(', ')}`)
        .toBeGreaterThanOrEqual(4);
      expect(produced).toContain('rest');
      expect(produced).toContain('active');
      expect(produced).toContain('selected');
      expect(produced).toContain('unavailable');

      for (const [index, cell] of facts.entries()) {
        const state = produced[index];
        if (state === undefined) continue;
        const spec = swatchStates[state];
        expect(cell.borderColor, `${state} cell ${String(index)} ring`).toBe(
          expectedColor(spec.cellRing, scheme),
        );
        expect(cell.borderStyle, `${state} cell ${String(index)} edge`).toBe(
          state === 'unavailable' ? 'dashed' : 'solid',
        );
        // The measured ring is a shadow and only the two selected states draw one.
        if (spec.insetRing) expect(cell.shadow, `${state} draws no inset ring`).not.toBe('none');
        else expect(cell.shadow, `${state} draws an inset ring it should not`).toBe('none');
      }
    });
  }

  test('no two states look alike, which is a different claim from the one above', async ({ page }) => {
    await open(page, holesStory);
    await settlePaint(page);
    const fingerprints = await page.locator('#partial-theme [role="option"]').evaluateAll((elements) => {
      const seen = new Map<string, string>();
      for (const element of elements) {
        const state =
          element.getAttribute('aria-disabled') === 'true'
            ? 'unavailable'
            : `${element.getAttribute('aria-selected') === 'true' ? 's' : ''}${element.hasAttribute('data-active') ? 'a' : ''}` || 'rest';
        const style = getComputedStyle(element);
        seen.set(state, `${style.borderTopColor}|${style.borderTopStyle}|${style.boxShadow !== 'none' ? 'ring' : 'flat'}`);
      }
      return [...seen.entries()];
    });
    const values = fingerprints.map(([, value]) => value);
    expect(new Set(values).size, `states looked alike: ${fingerprints.map(([k, v]) => `${k}=${v}`).join('  ')}`)
      .toBe(values.length);
  });

  test('the declared non-colour cue is actually drawn, not merely declared', async ({ page }) => {
    await open(page, themeStory);
    await settlePaint(page);
    // A cue that is declared and not rendered fails louder than no declaration at all.
    const marks = await page
      .locator('#document-theme [role="option"]')
      .evaluateAll((elements) =>
        elements.map((element) => ({
          selected: element.getAttribute('aria-selected') === 'true',
          markShown: getComputedStyle(element.querySelector('.swatch-mark') as Element).display !== 'none',
        })),
      );
    const selected = marks.filter((mark) => mark.selected);
    expect(selected.length, 'the story chose no swatch, so the cue has nowhere to be').toBe(1);
    for (const mark of marks) expect(mark.markShown).toBe(mark.selected);
    expect(swatchStates.selected.nonColourCue).toBeDefined();
  });
});

// ── the indicator, measured on what the browser actually composited ──────────

test.describe('the selected-swatch indicator', () => {
  for (const scheme of schemes) {
    test(`reads on every swatch in a real palette, in ${scheme}`, async ({ page }) => {
      await open(page, themeStory, { theme: scheme });
      await settlePaint(page);

      const measured = await page.locator('#document-theme [role="option"]').evaluateAll((elements) =>
        elements.map((element) => {
          const paint = element.querySelector('.swatch-paint') as HTMLElement;
          const mark = element.querySelector('.swatch-mark') as HTMLElement;
          return {
            label: element.getAttribute('aria-label') ?? '',
            // The *composited* colours: the fill the browser resolved, and the colour the ring and
            // the check mark inherit — which is `var(--mjx-swatch-indicator)` after substitution.
            fill: getComputedStyle(paint).backgroundColor,
            indicator: getComputedStyle(mark).color,
            empty: paint.hasAttribute('data-empty'),
          };
        }),
      );

      const painted = measured.filter((cell) => !cell.empty);
      // 60 theme + 10 standard + 3 recent + the Automatic chip, which the story gives a colour to.
      expect(painted.length, 'nothing in the palette was painted at all').toBeGreaterThanOrEqual(70);

      let worst = { ratio: Number.POSITIVE_INFINITY, at: '' };
      for (const cell of painted) {
        const fill = hexOf(cell.fill);
        const indicator = hexOf(cell.indicator);
        expect(fill, `${cell.label} reported an unreadable fill: ${cell.fill}`).toBeDefined();
        expect(indicator, `${cell.label} reported an unreadable indicator: ${cell.indicator}`).toBeDefined();
        const ratio = contrastRatioOrWorst(indicator ?? '', fill ?? '');
        if (ratio < worst.ratio) worst = { ratio, at: `${cell.label} (${fill ?? '?'})` };
      }
      expect(
        worst.ratio,
        `the weakest indicator in the live palette was ${formatRatio(worst.ratio)} on ${worst.at}`,
      ).toBeGreaterThanOrEqual(nonTextMinimum);
    });

    test(`a fixed indicator would have failed the same palette, in ${scheme}`, async ({ page }) => {
      // ⚠ The failability half, measured on the same live colours. Without it the assertion above
      // is satisfied by any palette that happens to be kind, and the whole design decision — that
      // the indicator is chosen per swatch — would be unfalsifiable.
      await open(page, themeStory, { theme: scheme });
      await settlePaint(page);
      const fills = await page
        .locator('#document-theme [role="option"] .swatch-paint')
        .evaluateAll((elements) =>
          elements
            .filter((element) => !element.hasAttribute('data-empty'))
            .map((element) => getComputedStyle(element).backgroundColor),
        );
      for (const member of ['accent', 'accentPressed', 'textPrimary', 'surface'] as const) {
        const fixed = tokens.theme[scheme][member];
        let lowest = Number.POSITIVE_INFINITY;
        for (const fill of fills) lowest = Math.min(lowest, contrastRatioOrWorst(fixed, hexOf(fill) ?? ''));
        expect(lowest, `a fixed ${member} indicator would have passed this palette`).toBeLessThan(
          nonTextMinimum,
        );
      }
    });
  }
});

// ── the value out is the value in ────────────────────────────────────────────

test.describe('the colour a person chose', () => {
  test('a theme swatch reports a slot, and never the colour it currently is', async ({ page }) => {
    await open(page, themeStory);
    await settlePaint(page);

    // Choose the sixth theme swatch on the third row: Accent 2, Lighter 60%.
    await page.locator('#document-theme [role="option"][aria-label="Accent 2, Lighter 60%"]').click();
    await settle(page);

    const state = await page.locator('#document-theme').evaluate((element) => ({
      value: (element as HTMLElement & { value: string }).value,
      text: (element.shadowRoot?.querySelector('input') as HTMLInputElement | null)?.value ?? '',
    }));

    expect(state.value).toBe('theme:accent2/lighter60');
    // The invariant every list field in this catalogue is built on: what the field shows is what
    // the control reports.
    expect(state.text).toBe('Accent 2, Lighter 60%');
    // And the value is a slot rather than a colour, structurally.
    const parsed = parseColorChoice(state.value);
    expect(parsed?.kind).toBe('theme');
    expect(Object.keys(parsed ?? {}).sort()).toEqual(['kind', 'slot', 'variant']);
    expect(state.value).not.toMatch(/#[0-9a-f]{6}/i);
  });

  test('every spelling of one colour commits one value — including one on no grid', async ({ page }) => {
    await open(page, offGridStory);
    await settle(page);

    const field = page.locator('#off-grid input');
    for (const spelling of ['#123457', '#123457'.toUpperCase(), 'rgb(18, 52, 87)', '123457']) {
      await field.click();
      await page.keyboard.press('ControlOrMeta+a');
      await field.fill(spelling);
      await page.keyboard.press('Enter');
      await settle(page);
      const state = await page.locator('#off-grid').evaluate((element) => ({
        value: (element as HTMLElement & { value: string }).value,
        text: (element.shadowRoot?.querySelector('input') as HTMLInputElement | null)?.value ?? '',
      }));
      expect(state.value, `${spelling} did not commit the canonical value`).toBe('#123457');
      expect(state.text, `${spelling} left the field disagreeing with the value`).toBe('#123457');
    }
  });

  test('a colour the grid does not carry still round-trips through the attribute', async ({ page }) => {
    await open(page, offGridStory);
    await settle(page);
    const roundTrip = await page.locator('#off-grid').evaluate((element) => {
      const picker = element as HTMLElement & { value: string };
      const before = picker.value;
      picker.value = 'theme:accent5/darker25';
      const middle = picker.value;
      picker.value = before;
      return { before, middle, after: picker.value };
    });
    expect(roundTrip.before).toBe('#123457');
    expect(roundTrip.middle).toBe('theme:accent5/darker25');
    expect(roundTrip.after).toBe('#123457');
    // And the model agrees about the same string, which is what makes the attribute a contract
    // rather than a place the component happens to keep something.
    const choice = parseColorChoice(roundTrip.middle) as ColorChoice;
    expect(formatColorChoice(choice)).toBe(roundTrip.middle);
  });

  test('a picker with no theme draws no theme section rather than inventing one', async ({ page }) => {
    await open(page, noThemeStory);
    await settlePaint(page);
    const headings = await page
      .locator('#no-theme .section-heading')
      .evaluateAll((elements) => elements.map((element) => element.textContent?.trim() ?? ''));
    expect(headings).not.toContain('Theme colours');
    // Ten standard colours and nothing else: the component supplied none of its own.
    await expect(page.locator('#no-theme [role="option"]')).toHaveCount(10);
  });

  test('a slot the document does not define is refused and says why', async ({ page }) => {
    await open(page, holesStory);
    await settlePaint(page);
    const refused = page.locator('#partial-theme [role="option"][aria-disabled="true"]');
    // Four gallery slots undefined, six variants each.
    await expect(refused).toHaveCount(24);
    const labels = await refused.evaluateAll((elements) =>
      elements.map((element) => element.getAttribute('aria-label') ?? ''),
    );
    for (const label of labels) expect(label).toContain('does not define');
  });
});

// ── the grid's keyboard ──────────────────────────────────────────────────────

test.describe('the palette’s keyboard', () => {
  test('the arrows walk the grid the way the model says, cell for cell', async ({ page }) => {
    await open(page, themeStory);
    await settlePaint(page);

    const start = await page.locator('#document-theme').evaluate((element) => {
      const picker = element as HTMLElement & { surface?: { activeIndex: number }; focus(): void };
      picker.focus();
      return picker.surface?.activeIndex ?? -1;
    });
    expect(start).toBeGreaterThanOrEqual(0);

    // The sections the component actually built, so the prediction is over the real grid.
    const categories = await page.locator('#document-theme [role="option"]').evaluateAll((elements) =>
      elements.map((element) => element.closest('.section-grid')?.previousElementSibling?.textContent?.trim() ?? ''),
    );
    const sections = swatchSections(
      categories.map((category, index) => ({ value: String(index), label: String(index), category })),
    );

    let predicted = start;
    for (const [key, action] of [
      ['ArrowRight', 'inlineNext'],
      ['ArrowRight', 'inlineNext'],
      ['ArrowDown', 'rowNext'],
      ['ArrowLeft', 'inlinePrevious'],
      ['ArrowUp', 'rowPrevious'],
      ['End', 'last'],
      ['Home', 'first'],
    ] as const) {
      predicted = nextSwatchIndex(action, predicted, sections);
      await page.keyboard.press(key);
      await settle(page);
      const actual = await page
        .locator('#document-theme')
        .evaluate((element) => (element as HTMLElement & { surface?: { activeIndex: number } }).surface?.activeIndex ?? -1);
      expect(actual, `${key} disagreed with the model`).toBe(predicted);
    }
  });

  test('the inline arrows mirror under right-to-left and the block ones do not', async ({ page }) => {
    await open(page, rtlStory);
    await settlePaint(page);
    const read = async (): Promise<number> =>
      page
        .locator('#rtl-palette')
        .evaluate((element) => (element as HTMLElement & { surface?: { activeIndex: number } }).surface?.activeIndex ?? -1);

    await page.locator('#rtl-palette').evaluate((element) => (element as HTMLElement).focus());
    const before = await read();
    await page.keyboard.press('ArrowRight');
    await settle(page);
    const afterRight = await read();
    // ⚠ Asserted as a *direction*, not as a landing place: under right-to-left, Arrow Right must
    // move the cursor **backwards** through the reading order. An assertion that it ended up on a
    // cell would pass on an implementation that never mirrored at all.
    expect(afterRight, 'Arrow Right did not mirror').toBeLessThan(before);

    await page.keyboard.press('ArrowDown');
    await settle(page);
    const afterDown = await read();
    expect(afterDown, 'Arrow Down mirrored, and the block axis never does').toBeGreaterThan(afterRight);
  });

  test('the whole picker is one tab stop, and Tab out closes the palette', async ({ page }) => {
    await open(page, themeStory);
    await settle(page);
    await page.locator('#document-theme').evaluate((element) => (element as HTMLElement).focus());
    expect((await deepActive(page)).role).toBe('combobox');
    await page.keyboard.press('Tab');
    await settle(page);
    expect((await deepActive(page)).role).not.toBe('option');
    const open_ = await page
      .locator('#document-theme')
      .evaluate((element) => (element as HTMLElement & { open: boolean }).open);
    expect(open_).toBe(false);
  });

  test('a swatch never takes focus, which is what makes the stop count one', async ({ page }) => {
    await open(page, themeStory);
    await settle(page);
    const focusables = await page
      .locator('#document-theme [role="option"]')
      .evaluateAll((elements) => elements.filter((element) => (element as HTMLElement).tabIndex >= 0).length);
    expect(focusables).toBe(0);
    const stops = await tabStops(page);
    expect(stops.filter((stop) => stop.includes('[option]'))).toEqual([]);
  });

  test('the placement is the one placeFloating computes in Node from the same inputs', async ({ page }) => {
    await open(page, flipStory);
    await settlePaint(page);
    const recorded = await page.locator('#flipping-palette').evaluate((element) => {
      const surface = (
        element as HTMLElement & {
          surface?: {
            placement?: unknown;
            anchorRect?: unknown;
            naturalSize?: unknown;
            boundaryRect?: unknown;
          };
        }
      ).surface;
      return {
        placement: surface?.placement as { x: number; y: number; side: string; flipped: boolean } | undefined,
        anchor: surface?.anchorRect as Rect | undefined,
        natural: surface?.naturalSize as { width: number; height: number } | undefined,
        boundary: surface?.boundaryRect as Rect | undefined,
      };
    });
    expect(recorded.placement, 'the palette recorded no placement').toBeDefined();
    expect(recorded.anchor).toBeDefined();
    expect(recorded.natural).toBeDefined();
    expect(recorded.boundary).toBeDefined();
    if (
      recorded.placement === undefined ||
      recorded.anchor === undefined ||
      recorded.natural === undefined ||
      recorded.boundary === undefined
    ) {
      return;
    }
    const again = placeFloating({
      anchor: recorded.anchor,
      floating: recorded.natural,
      boundary: recorded.boundary,
      side: 'blockEnd',
      align: 'start',
      direction: 'ltr',
      gap: 4,
    });
    expect(again.side).toBe(recorded.placement.side);
    expect(again.flipped).toBe(recorded.placement.flipped);
    // The story puts the field at the foot of what clips it, so this must actually have flipped —
    // otherwise the agreement above is an agreement about the easy case.
    expect(recorded.placement.flipped).toBe(true);
  });
});

// ── the swatch cell's size ───────────────────────────────────────────────────

test('a swatch cell clears the hit-target floor in compact density', async ({ page }) => {
  await open(page, compactColourStory);
  await settlePaint(page);
  const boxes = await page
    .locator('#compact-palette [role="option"]')
    .evaluateAll((elements) => elements.map((element) => element.getBoundingClientRect()));
  expect(boxes.length).toBeGreaterThan(0);
  for (const box of boxes) {
    expect(box.width, 'a compact swatch is narrower than the floor').toBeGreaterThanOrEqual(
      accessibleHitTargetMinimum,
    );
    expect(box.height, 'a compact swatch is shorter than the floor').toBeGreaterThanOrEqual(
      accessibleHitTargetMinimum,
    );
  }
  // Anti-vacuity: compact must actually be smaller than comfortable, or this measures a constant.
  const comfortable = await page.evaluate(() => {
    const probe = document.createElement('div');
    probe.className = 'mjx-hit-target';
    document.body.append(probe);
    const height = probe.getBoundingClientRect().height;
    probe.remove();
    return height;
  });
  expect(boxes[0]?.height ?? 0).toBeLessThan(comfortable);
});

// ── the font picker's warning ────────────────────────────────────────────────

test.describe('the substitution warning', () => {
  test('it appears for a substituted face and does not for a present one', async ({ page }) => {
    await open(page, warningStory);
    await settle(page);

    const read = async (id: string): Promise<{ note: string; described: string | null; shown: boolean }> =>
      page.locator(id).evaluate((element) => {
        const root = element.shadowRoot;
        const message = root?.querySelector('.substitution-message') as HTMLElement | null;
        const entry = root?.querySelector('input') as HTMLInputElement | null;
        return {
          note: message?.textContent ?? '',
          described: entry?.getAttribute('aria-describedby') ?? null,
          shown: message !== null && !message.hidden,
        };
      });

    const present = await read('#warning-present');
    const absent = await read('#warning-absent');

    // ⚠ The headline assertion of this control, in both directions in one test — because the two
    // fields are indistinguishable without it, and that indistinguishability *is* the failure.
    expect(present.shown, 'a substituted family showed no warning').toBe(true);
    expect(present.note).toContain('Caladea');
    expect(present.described).toBe('substitution');
    expect(absent.shown, 'an installed family showed a warning').toBe(false);
    expect(absent.note).toBe('');
    expect(absent.described).toBeNull();

    // And it is the model's own sentence rather than a second one written in the component.
    expect(present.note).toBe(
      substitutionNote({
        family: 'Cambria',
        availability: 'substituted',
        substitutedBy: 'Caladea',
        metricCompatible: true,
      }),
    );
  });

  test('a metric-compatible substitution and a reflowing one say different things', async ({ page }) => {
    await open(page, fontStatesStory);
    await settle(page);
    const notes = await page
      .locator('mjx-font-picker')
      .evaluateAll((elements) =>
        elements.map(
          (element) => element.shadowRoot?.querySelector('.substitution-message')?.textContent ?? '',
        ),
      );
    const said = notes.filter((sentence) => sentence !== '');
    expect(said.length).toBeGreaterThanOrEqual(2);
    expect(new Set(said).size, 'two different facts produced one sentence').toBe(said.length);
    expect(said.some((sentence) => sentence.includes('metrics match'))).toBe(true);
    expect(said.some((sentence) => sentence.includes('fallback') || sentence.includes('metrics differ'))).toBe(
      true,
    );
  });

  test('the row carries the mark, and it is told apart by size and never by colour', async ({ page }) => {
    await open(page, rowsStory);
    await settlePaint(page);

    const rows = await page.locator('#open-list [role="option"]').evaluateAll((elements) =>
      elements.map((element) => {
        const label = element.querySelector('.option-label') as HTMLElement | null;
        const mark = element.querySelector('.substitution') as HTMLElement | null;
        return {
          family: element.getAttribute('aria-posinset') ?? '',
          labelColor: label === null ? '' : getComputedStyle(label).color,
          labelSize: label === null ? 0 : Number.parseFloat(getComputedStyle(label).fontSize),
          markColor: mark === null ? null : getComputedStyle(mark).color,
          markSize: mark === null ? 0 : Number.parseFloat(getComputedStyle(mark).fontSize),
          markText: mark?.textContent?.trim() ?? '',
        };
      }),
    );

    const marked = rows.filter((row) => row.markColor !== null);
    expect(marked.length, 'no row in the window carries a mark').toBeGreaterThan(0);
    for (const row of marked) {
      expect(row.markText === 'Substituted' || row.markText === 'Missing').toBe(true);
      // Same colour as the label beside it, smaller size. Grey here would be 4.32 : 1 the moment
      // the keyboard cursor filled the row.
      expect(row.markColor, 'a substitution mark is a weaker colour than its label').toBe(row.labelColor);
      expect(row.markSize, 'a substitution mark is not smaller than its label').toBeLessThan(row.labelSize);
    }
  });

  test('a substituted family is still choosable, and choosing it commits it', async ({ page }) => {
    await open(page, rowsStory);
    await settlePaint(page);
    const refused = await page.locator('#open-list [role="option"][aria-disabled="true"]').count();
    expect(refused, 'a substituted family was refused').toBe(0);

    // ⚠ Whichever marked row the window happens to hold, rather than a family named here. The list
    // is virtualised, so a named row may simply not be in the DOM — and a test that waited for one
    // would hang rather than fail, which is the worst way for a gate to be wrong.
    //
    // Located by a **filter that is re-evaluated**, and never by an id read out once: a list
    // surface rebuilds its window whenever the cursor moves, so an element captured a moment ago
    // may not be the element in the tree now. Held by id, this click timed out in the full suite
    // and passed on its own — the signature of a stale handle rather than of a defect.
    const markedRow = (): ReturnType<typeof page.locator> =>
      page
        .locator('#open-list [role="option"]')
        .filter({ has: page.locator('.substitution') })
        .first();
    await expect(markedRow(), 'no substituted family is in the window at all').toBeVisible();

    // ⚠ **Scroll first, settle, and only then read the row and press it.**
    //
    // A virtualised list re-renders on scroll, so the element that was under the pointer a moment
    // ago is detached by the time the press lands — and `ListSurface` resolves a press by element
    // identity, so a press on a detached row commits nothing at all. That is what this assertion
    // caught: the click "worked", the list still showed the old value, and the failure said only
    // that two family names disagreed. Bringing the row into view *before* reading it means the
    // window the press lands in is the window that was measured.
    await markedRow().scrollIntoViewIfNeeded();
    await settle(page);
    const marked = { family: (await markedRow().locator('.option-label').textContent())?.trim() ?? '' };
    expect(marked.family).not.toBe('');

    await markedRow().click();
    await settle(page);
    const state = await page.locator('#open-list').evaluate((element) => ({
      value: (element as HTMLElement & { value: string }).value,
      text: (element.shadowRoot?.querySelector('input') as HTMLInputElement | null)?.value ?? '',
      warned: (element as HTMLElement & { warning?: string }).warning !== undefined,
    }));
    expect(state.value).toBe(marked.family);
    expect(state.text).toBe(marked.family);
    // And choosing it turns the field's own warning on, which is the round trip that matters:
    // the list said the face was substituted, and the field now says the same thing.
    expect(state.warned).toBe(true);
  });
});

// ── U06's precondition, walked into on purpose ───────────────────────────────

test.describe('a list of faces', () => {
  test('every row is exactly one row tall, headings included', async ({ page }) => {
    await open(page, facesStory);
    await settlePaint(page);

    const measured = await page.locator('#long-faces .list').evaluate((list) => {
      const rows = [...list.querySelectorAll('.option')].map((row) => row.getBoundingClientRect().height);
      const headings = [...list.querySelectorAll('.section')].map((row) => row.getBoundingClientRect().height);
      const families = [...list.querySelectorAll('.option-label')].map(
        (label) => getComputedStyle(label).fontFamily,
      );
      const leadings = [...list.querySelectorAll('.option-label')].map(
        (label) => getComputedStyle(label).lineHeight,
      );
      // The height a row is *supposed* to be, from somewhere other than a row: the foundations'
      // own hit-target floor, measured on a probe. A custom property cannot be read back resolved,
      // and comparing rows only with each other is what would let a row lose its height entirely
      // and stay green — every row would simply be wrong together.
      const probe = document.createElement('div');
      probe.className = 'mjx-hit-target';
      list.append(probe);
      const expected = probe.getBoundingClientRect().height;
      probe.remove();
      return { rows, headings, families, leadings, expected };
    });

    expect(measured.rows.length, 'the list built no rows').toBeGreaterThan(4);
    expect(measured.headings.length, 'the window contains no heading, so the hard half is untested')
      .toBeGreaterThan(0);

    // ⚠ Anti-vacuity, and the reason this fixture uses generic families. If every row were drawn
    // in one face, "all rows are the same height" would be a tautology about a list of one face.
    expect(new Set(measured.families).size, `only ${new Set(measured.families).size} face(s) in the list`)
      .toBeGreaterThanOrEqual(3);
    // ⚠ And the second half of the same argument. With a *fixed* leading no face could change a
    // row's height, so the assertion below would hold with the row's own block-size deleted and
    // this gate would prove nothing. The preview uses the face's own line box — which a browser
    // reports as the string `normal` rather than as a resolved length, so this is the property
    // being asserted rather than the five different heights it produces.
    expect(new Set(measured.leadings), 'the preview leading is fixed, so the gate proves nothing')
      .toEqual(new Set(['normal']));

    const heights = new Set([...measured.rows, ...measured.headings].map((height) => height.toFixed(2)));
    expect(
      heights.size,
      `rows came out at ${[...heights].join(', ')} — the virtualiser divides by one of them`,
    ).toBe(1);
    // Equal to each other *and* equal to the height the density mode declares.
    expect([...heights][0]).toBe(measured.expected.toFixed(2));
  });

  test('the faces in the fixture genuinely differ, which is what the gate above depends on', async ({
    page,
  }) => {
    await open(page, facesStory);
    await settlePaint(page);
    const probes = await page
      .locator('[data-face-probe]')
      .evaluateAll((elements) =>
        elements.map((element) => ({
          family: element.getAttribute('data-face-probe') ?? '',
          height: element.getBoundingClientRect().height,
          width: element.getBoundingClientRect().width,
        })),
      );
    expect(probes.length).toBeGreaterThanOrEqual(5);
    // Outside the list, with nothing holding their height, the same five faces are visibly
    // different heights. That is what a row would be if the row did not decide its own height.
    // Width rather than height, and deliberately: a face's advance widths differ on every machine
    // this catalogue runs on, whereas its line box may be normalised by whatever fontconfig maps
    // the generic families to. A gate that is only true on one machine is a gate that will be
    // deleted by whoever meets it on another.
    const widths = new Set(probes.map((probe) => probe.width.toFixed(2)));
    const heights = new Set(probes.map((probe) => probe.height.toFixed(2)));
    expect(
      widths.size,
      `all five probe faces measured ${[...widths].join(', ')} wide and ${[...heights].join(', ')} tall`,
    ).toBeGreaterThan(1);
  });

  test('only the window is built, and the number comes from the shared planner', async ({ page }) => {
    await open(page, facesStory);
    await settlePaint(page);
    const state = await page.locator('#long-faces').evaluate((element) => {
      const picker = element as HTMLElement & {
        options: { category?: string }[];
        surface?: {
          builtRowCount: number;
          expectedCellCount: number;
          builtWindow: { firstRow: number; lastRow: number };
        };
      };
      return {
        total: picker.options.length,
        categories: picker.options.map((option) => option.category ?? ''),
        built: picker.surface?.builtRowCount ?? -1,
        expected: picker.surface?.expectedCellCount ?? -1,
        window: picker.surface?.builtWindow ?? { firstRow: -1, lastRow: -1 },
      };
    });

    expect(state.total).toBe(60);
    expect(state.built).toBeGreaterThan(0);
    expect(state.built, 'the whole list was built, so nothing is virtualised').toBeLessThan(state.total);
    // The number comes from somewhere other than the code that built the nodes — U06's own
    // planner, re-run here over the categories the component is actually holding.
    const plan = galleryRowPlan(state.categories, 1, true);
    expect(state.built).toBe(cellsInWindow(plan, state.window));
    expect(state.built).toBe(state.expected);
  });

  test('a compact list is shorter and still one height throughout', async ({ page }) => {
    await open(page, compactFontStory);
    await settlePaint(page);
    const heights = await page.locator('#compact-faces .list').evaluate((list) => {
      const all = [...list.querySelectorAll('.option'), ...list.querySelectorAll('.section')];
      return all.map((row) => row.getBoundingClientRect().height);
    });
    expect(heights.length).toBeGreaterThan(4);
    expect(new Set(heights.map((height) => height.toFixed(2))).size).toBe(1);
    expect(heights[0] ?? 0).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
  });

  test('the font picker is one tab stop as well', async ({ page }) => {
    await open(page, rowsStory);
    await settle(page);
    await page.locator('#open-list').evaluate((element) => (element as HTMLElement).focus());
    expect((await deepActive(page)).role).toBe('combobox');
    const stops = await tabStops(page, 4);
    expect(stops.filter((stop) => stop.includes('[option]'))).toEqual([]);
  });
});
