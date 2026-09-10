import { expect, test, type Locator, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import { accessibleHitTargetMinimum } from '../../src/foundations/density.ts';
import {
  boxPaint,
  boxStateCellAttribute,
  boxStateNames,
  cellsInWindow,
  fieldPaint,
  fieldStateCellAttribute,
  fieldStateNames,
  filterOptions,
  galleryRowPlan,
  inputStatesMatrixStoryName,
  inputStoryTitles,
  listboxVisibleRows,
  optionPaint,
  optionStateOf,
  sliderFraction,
  snapToStep,
} from '../../src/inputs/input-model.ts';
import type { EffectiveStatePaint } from '../../src/controls/control-states.ts';
import { formatMeasure, parseMeasure, pointsFrom } from '../../src/inputs/measure.ts';
import { placeFloating, type Rect } from '../../src/overlay/floating.ts';

/**
 * The inputs, measured and driven in a real browser.
 *
 * MJXOFF-186 names four traps and every one of them is a trap because it is **invisible to a
 * static story**, so almost nothing below is a snapshot:
 *
 * * a combo box whose field and value disagree looks perfect and is unusable;
 * * a checkbox whose third state never rendered has two states and a story that shows two;
 * * a slider tested at one value exercises no arithmetic at all;
 * * a measure input that silently reverts is indistinguishable from one that committed.
 *
 * And MJXOFF-182's rule decides the shape of the gates that *are* about appearance:
 *
 * > **A distinctness gate proves no two states are the same; it does not prove any of them is
 * > right.**
 *
 * So every paint suite asserts **correspondence** — a measured colour equals what the model says —
 * and distinctness separately, and the placement suite re-runs `placeFloating()` in Node over the
 * component's own recorded inputs.
 */

const measureInput = inputStoryTitles.measureInput;
const checkbox = inputStoryTitles.checkbox;
const dropdown = inputStoryTitles.dropdown;
const comboBox = inputStoryTitles.comboBox;
const slider = inputStoryTitles.slider;
const segmented = inputStoryTitles.segmentedControl;
const label = inputStoryTitles.label;

const fieldMatrixStory = { title: measureInput, name: inputStatesMatrixStoryName } as const;
const boxMatrixStory = { title: checkbox, name: inputStatesMatrixStoryName } as const;
const threePositionsStory = { title: checkbox, name: 'The Three Positions' } as const;
const optionStatesStory = { title: dropdown, name: 'The Option States' } as const;
const longListStory = { title: dropdown, name: 'A Long List' } as const;
const flipStory = { title: dropdown, name: 'At The Bottom Of Its Room' } as const;
const closedFieldStory = { title: dropdown, name: 'The Closed Field' } as const;
const filterStory = { title: comboBox, name: 'Typing Filters' } as const;
const customStory = { title: comboBox, name: 'A String The List Does Not Carry' } as const;
const paneStory = { title: comboBox, name: 'In A Properties Pane' } as const;
const nonsenseStory = { title: measureInput, name: 'What It Does With Nonsense' } as const;
const unitsStory = { title: measureInput, name: 'The Same Quantity Six Ways' } as const;
const commaStory = { title: measureInput, name: 'With A Decimal Comma' } as const;
const rangeStory = { title: measureInput, name: 'Stepping Inside A Range' } as const;
const valuesStory = { title: slider, name: 'At Several Values' } as const;
const offBoundaryStory = { title: slider, name: 'A Maximum Off The Boundary' } as const;
const fractionalStory = { title: slider, name: 'A Fractional Step' } as const;
const sliderRtlStory = { title: slider, name: 'Under Right To Left' } as const;
const alignmentStory = { title: segmented, name: 'The Alignment Set' } as const;
const unavailableSegmentStory = { title: segmented, name: 'A Member That Cannot Be Chosen' } as const;
const namingStory = { title: label, name: 'What It Actually Does' } as const;
const compactCheckboxStory = { title: checkbox, name: 'In Compact Density' } as const;

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
  await page.waitForFunction(() => customElements.get('mjx-measure-input') !== undefined);
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

interface Paint {
  background: string;
  borderColor: string;
  borderStyle: string;
  color: string;
  weight: string;
  opacity: string;
  shadow: string;
}

async function paintOf(locator: Locator): Promise<Paint> {
  return locator.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      background: style.backgroundColor,
      borderColor: style.borderTopColor,
      borderStyle: style.borderTopStyle,
      color: style.color,
      weight: style.fontWeight,
      opacity: style.opacity,
      shadow: style.boxShadow,
    };
  });
}

function expectedPaint(
  paint: EffectiveStatePaint,
  scheme: ColorScheme,
): Omit<Paint, 'shadow' | 'opacity'> & { opacity: string } {
  return {
    background: expectedColor(paint.background, scheme),
    borderColor: expectedColor(paint.borderColor, scheme),
    borderStyle: paint.borderStyle,
    color: expectedColor(paint.text, scheme),
    weight: paint.weight === 'bold' ? String(tokens.fontWeight.bold) : String(tokens.fontWeight.medium),
    opacity: String(paint.opacity),
  };
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

/**
 * What a fixed number of `Tab` presses lands on, in order.
 *
 * ⚠ **It does not stop early**, and the first version of this did. A landing on `body` — which is
 * what the very first press produces when nothing was focused — ended the walk and returned an
 * empty list, and the gate then reported *zero* tab stops for a pane that has two. A helper whose
 * failure mode is "found nothing" is a helper that makes every ceiling assertion pass, so it
 * presses a fixed number of times and reports every landing including the empty ones.
 */
async function tabStops(page: Page, presses = 10): Promise<string[]> {
  const seen: string[] = [];
  for (let index = 0; index < presses; index += 1) {
    await page.keyboard.press('Tab');
    const active = await deepActive(page);
    seen.push(`${active.tag}[${active.role}]${active.name === '' ? '' : `:${active.name}`}`);
  }
  return seen;
}

// ── the field states ─────────────────────────────────────────────────────────

test.describe('a field’s paint', () => {
  for (const scheme of schemes) {
    test(`every cell computes what the model says, in ${scheme}`, async ({ page }) => {
      await open(page, fieldMatrixStory, { theme: scheme });
      await settlePaint(page);

      const cells = page.locator(`[${fieldStateCellAttribute}]`);
      // The count first: a divergence between the story and the model fails with a number rather
      // than by sweeping an empty list and reporting nothing.
      await expect(cells).toHaveCount(fieldStateNames.length);

      for (const state of fieldStateNames) {
        const field = page.locator(`[${fieldStateCellAttribute}="${state}"] .field`);
        await expect(field, `${state} has no field`).toHaveCount(1);
        const measured = await paintOf(field);
        const wanted = expectedPaint(fieldPaint(state), scheme);
        expect(
          {
            background: measured.background,
            borderColor: measured.borderColor,
            borderStyle: measured.borderStyle,
            opacity: measured.opacity,
          },
          `${state} in ${scheme}`,
        ).toEqual({
          background: wanted.background,
          borderColor: wanted.borderColor,
          borderStyle: wanted.borderStyle,
          opacity: wanted.opacity,
        });
      }
    });

    test(`no two cells look alike, in ${scheme}`, async ({ page }) => {
      await open(page, fieldMatrixStory, { theme: scheme });
      await settlePaint(page);
      const seen = new Map<string, string>();
      for (const state of fieldStateNames) {
        const measured = await paintOf(page.locator(`[${fieldStateCellAttribute}="${state}"] .field`));
        const print = JSON.stringify(measured);
        const clash = seen.get(print);
        // `focus` and `rest` are deliberately identical in paint — the ring is the difference and
        // it is the foundations', so it is not on this element.
        if (state === 'focus' || clash === 'focus') continue;
        expect(clash, `${state} and ${String(clash)} render identically in ${scheme}`).toBeUndefined();
        seen.set(print, state);
      }
    });
  }

  test('the fill never changes, which is the reason the table exists', async ({ page }) => {
    await open(page, fieldMatrixStory);
    await settlePaint(page);
    const fills = new Set<string>();
    for (const state of fieldStateNames) {
      if (state === 'disabled') continue;
      const measured = await paintOf(page.locator(`[${fieldStateCellAttribute}="${state}"] .field`));
      fills.add(measured.background);
    }
    expect([...fills], 'a field state changed the fill').toHaveLength(1);
  });
});

// ── the checkbox's three positions ───────────────────────────────────────────

test.describe('a checkbox', () => {
  test('the box computes what the model says, for all four positions', async ({ page }) => {
    await open(page, boxMatrixStory);
    await settlePaint(page);
    const cells = page.locator(`[${boxStateCellAttribute}]`);
    await expect(cells).toHaveCount(boxStateNames.length);

    for (const state of boxStateNames) {
      const box = page.locator(`[${boxStateCellAttribute}="${state}"] .box`);
      await expect(box, `${state} has no box`).toHaveCount(1);
      const measured = await paintOf(box);
      const wanted = expectedPaint(boxPaint(state), 'light');
      expect({ background: measured.background, borderColor: measured.borderColor }, state).toEqual({
        background: wanted.background,
        borderColor: wanted.borderColor,
      });
    }
  });

  test('the third position is an attribute value and is announced as one', async ({ page }) => {
    await open(page, threePositionsStory);
    const checkboxes = page.locator('mjx-checkbox');
    await expect(checkboxes).toHaveCount(3);
    const announced = await page.locator('mjx-checkbox button[role="checkbox"]').evaluateAll(
      (buttons) => buttons.map((button) => button.getAttribute('aria-checked')),
    );
    // ⚠ Three *different* values, and `mixed` among them. A story that showed two would satisfy
    // "every checkbox announces something"; only the set assertion catches it.
    expect(announced).toEqual(['false', 'true', 'mixed']);
    expect(new Set(announced).size).toBe(3);
  });

  test('activating the indeterminate one checks it, and says so once', async ({ page }) => {
    await open(page, threePositionsStory);
    const events = await page.evaluate(() => {
      const seen: unknown[] = [];
      document.addEventListener('mjx-input-change', (event) => {
        seen.push((event as CustomEvent).detail);
      });
      (globalThis as { __events?: unknown[] }).__events = seen;
      return seen.length;
    });
    expect(events).toBe(0);

    const third = page.locator('mjx-checkbox').nth(2);
    await third.locator('button').click();
    const detail = await page.evaluate(() => (globalThis as { __events?: unknown[] }).__events);
    expect(detail).toEqual([{ value: 'true', previous: 'mixed' }]);
    await expect(third.locator('button')).toHaveAttribute('aria-checked', 'true');
  });

  test('the row clears the hit-target floor in compact density', async ({ page }) => {
    await open(page, compactCheckboxStory);
    await settle(page);
    const boxes = await page.locator('mjx-checkbox button').evaluateAll((buttons) =>
      buttons.map((button) => button.getBoundingClientRect().height),
    );
    expect(boxes.length).toBeGreaterThan(0);
    for (const height of boxes) expect(height).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
  });

  test('the two kinds of unavailability differ in the way the shared helper says', async ({ page }) => {
    await open(page, { title: checkbox, name: 'Unavailable And Disabled' });
    const buttons = page.locator('mjx-checkbox button');
    await expect(buttons).toHaveCount(2);
    // Explained: aria-disabled, still focusable, carrying its reason.
    await expect(buttons.nth(0)).toHaveAttribute('aria-disabled', 'true');
    expect(await buttons.nth(0).evaluate((button) => (button as HTMLButtonElement).disabled)).toBe(false);
    await expect(buttons.nth(0)).toHaveAttribute('aria-describedby', 'explanation');
    // Hard: the platform's own, and therefore *no* aria-disabled — the widening of
    // `applyAvailability` must not have changed this for an element that has the native property.
    expect(await buttons.nth(1).evaluate((button) => (button as HTMLButtonElement).disabled)).toBe(true);
    await expect(buttons.nth(1)).not.toHaveAttribute('aria-disabled', 'true');
  });
});

// ── the option states, on live rows ──────────────────────────────────────────

test.describe('an option', () => {
  test('all five states are produced at once, and each computes what the model says', async ({ page }) => {
    await open(page, optionStatesStory);
    await settlePaint(page);

    const rows = page.locator('#option-states [role="option"]');
    await expect(rows).toHaveCount(5);

    const facts = await rows.evaluateAll((elements) =>
      elements.map((element) => ({
        selected: element.getAttribute('aria-selected') === 'true',
        active: element.hasAttribute('data-active'),
        unavailable: element.getAttribute('aria-disabled') === 'true',
        background: getComputedStyle(element).backgroundColor,
        borderColor: getComputedStyle(element).borderTopColor,
        weight: getComputedStyle(element).fontWeight,
      })),
    );

    const produced = facts.map((row) => optionStateOf(row));
    // ⚠ Anti-vacuity, and the whole reason this story is arranged as it is: four *distinct* states
    // must be on screen. A story where every row was resting would satisfy every per-row assertion
    // below and prove nothing at all.
    expect(new Set(produced).size, `only produced ${produced.join(', ')}`).toBeGreaterThanOrEqual(4);
    expect(produced).toContain('selected');
    expect(produced).toContain('active');
    expect(produced).toContain('unavailable');
    expect(produced).toContain('rest');

    for (const [index, row] of facts.entries()) {
      const state = produced[index];
      if (state === undefined) continue;
      const wanted = expectedPaint(optionPaint(state), 'light');
      expect({ background: row.background, borderColor: row.borderColor }, `${state} row ${String(index)}`)
        .toEqual({ background: wanted.background, borderColor: wanted.borderColor });
      expect(row.weight, `${state} weight`).toBe(wanted.weight);
    }
  });

  test('the fifth state is reachable, and it is the one with two indicators', async ({ page }) => {
    await open(page, optionStatesStory);
    await settlePaint(page);
    /*
     * ⚠ **The precondition, asserted rather than assumed** — added by MJXOFF-191, which broke this
     * test without touching a line of it.
     *
     * The story opens its own list from a `requestAnimationFrame`, and the two presses below only
     * mean *move the cursor* while that list is open: on a **closed** dropdown an arrow key changes
     * the value instead. So a page that took a little longer to reach that frame — U12 added four
     * elements to `preview.ts` and twelve stories to the catalogue — turned this into two presses
     * that edited a value and a locator that then matched nothing. The failure named a row count and
     * said nothing at all about the list being shut.
     *
     * Waiting for the state the presses require is not a retry: it is the sentence the test's own
     * comment already assumed, made checkable.
     */
    const field = page.locator('#option-states button[role="combobox"]');
    await expect(field).toHaveAttribute('aria-expanded', 'true');
    await expect(page.locator('#option-states [role="option"][data-active]')).toHaveCount(1);
    // Arrow the cursor onto the row that is already chosen.
    await field.focus();
    // The story opens with the chosen row at index 1 and the cursor at 3, so two presses bring
    // them together. Stated as a number rather than looped until they coincide: a loop that ran
    // until the assertion held would be a loop that could never fail.
    await page.keyboard.press('ArrowUp');
    await page.keyboard.press('ArrowUp');
    await settlePaint(page);
    const found = await page.locator('#option-states [role="option"][aria-selected="true"][data-active]').count();
    expect(found).toBe(1);
    const measured = await paintOf(
      page.locator('#option-states [role="option"][aria-selected="true"][data-active]'),
    );
    const wanted = expectedPaint(optionPaint('selectedActive'), 'light');
    expect(measured.borderColor).toBe(wanted.borderColor);
    // The ring is the cursor. Its presence is the difference from `selected`.
    expect(measured.shadow).not.toBe('none');
  });

  test('a second line is told apart from its label by size and never by colour', async ({ page }) => {
    await open(page, optionStatesStory);
    await settlePaint(page);
    // ⚠ The claim is *within a row*, not across the list: `unavailable` is legitimately grey,
    // because that is the shared table's paint for it and the whole row goes grey together. What
    // must never happen is a description that is a weaker colour than the label beside it — which
    // is the thing that would be 4.32 : 1 the moment the keyboard cursor filled the row.
    const pairs = await page.locator('#option-states [role="option"]').evaluateAll((rows) =>
      rows
        .map((row) => {
          const labelPart = row.querySelector('.option-label');
          const description = row.querySelector('.option-description');
          if (labelPart === null || description === null) return null;
          return {
            labelColor: getComputedStyle(labelPart).color,
            descriptionColor: getComputedStyle(description).color,
            labelSize: Number.parseFloat(getComputedStyle(labelPart).fontSize),
            descriptionSize: Number.parseFloat(getComputedStyle(description).fontSize),
          };
        })
        .filter((pair) => pair !== null),
    );
    // Anti-vacuity: the story has to contain a row with a second line, or this measures nothing.
    expect(pairs.length, 'no option in the story has a second line').toBeGreaterThan(0);
    for (const pair of pairs) {
      expect(pair.descriptionColor, 'a second line is a weaker colour than its label').toBe(
        pair.labelColor,
      );
      expect(pair.descriptionSize, 'a second line is not smaller than its label').toBeLessThan(
        pair.labelSize,
      );
    }
  });
});

// ── the tab-stop doctrine ────────────────────────────────────────────────────

test.describe('the tab stops', () => {
  test('a properties pane of two fields has two stops, and the labels have none', async ({ page }) => {
    await open(page, paneStory);
    await settle(page);
    const stops = await tabStops(page);
    // Ten presses through a pane holding two labels and two fields. Named, because the harness's
    // own width control is a nameless `<input>` in the same walk and is not this pane's; the pane's
    // two fields are named exactly by the two labels above them, which is the other half of what
    // `<mjx-label>` is for. Asserted as a *set*, so a walk that went round twice says the same
    // thing as a walk that went round once.
    const named = new Set(stops.filter((stop) => stop.includes(':')));
    expect([...named].sort()).toEqual(['input[]:Size', 'input[combobox]:Font']);
    // …and a label is never one of them, which is itself this catalogue's audit finding about them.
    expect(stops.some((stop) => stop.startsWith('mjx-label'))).toBe(false);
  });

  test('an open list is a disclosure: Tab leaves it and it closes', async ({ page }) => {
    await open(page, optionStatesStory);
    await settle(page);
    const field = page.locator('#option-states button[role="combobox"]');
    await field.focus();
    await page.keyboard.press('ArrowDown');
    await settle(page);
    await expect(field).toHaveAttribute('aria-expanded', 'true');

    await page.keyboard.press('Tab');
    await settle(page);
    await expect(field).toHaveAttribute('aria-expanded', 'false');
    const active = await deepActive(page);
    expect(active.role, 'Tab left the field but stayed inside the control').not.toBe('combobox');
  });

  test('a segmented control holds exactly one stop for the whole group', async ({ page }) => {
    await open(page, alignmentStory);
    await settle(page);
    const tabbable = await page
      .locator('#alignment button[role="radio"]')
      .evaluateAll((buttons) => buttons.map((button) => (button as HTMLButtonElement).tabIndex));
    expect(tabbable).toHaveLength(4);
    expect(tabbable.filter((index) => index === 0)).toHaveLength(1);
  });
});

// ── the dropdown's keyboard ──────────────────────────────────────────────────

test.describe('a dropdown', () => {
  test('Home and End move through the list, because this field is not a text box', async ({ page }) => {
    await open(page, optionStatesStory);
    await settle(page);
    const field = page.locator('#option-states button[role="combobox"]');
    await field.focus();
    await page.keyboard.press('ArrowDown');
    await settle(page);

    await page.keyboard.press('End');
    await settle(page);
    let active = await field.getAttribute('aria-activedescendant');
    const last = await page.locator('#option-states [role="option"]').last().getAttribute('id');
    expect(active).toBe(last);

    await page.keyboard.press('Home');
    await settle(page);
    active = await field.getAttribute('aria-activedescendant');
    const first = await page.locator('#option-states [role="option"]').first().getAttribute('id');
    expect(active).toBe(first);
  });

  test('Enter commits the option the cursor is on; Escape commits nothing', async ({ page }) => {
    await open(page, closedFieldStory);
    await settle(page);
    const control = page.locator('mjx-dropdown').first();
    const field = control.locator('button[role="combobox"]');

    const before = await control.getAttribute('value');
    await field.focus();
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Escape');
    await settle(page);
    expect(await control.getAttribute('value'), 'Escape committed something').toBe(before);
    await expect(field).toHaveAttribute('aria-expanded', 'false');

    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    await settle(page);
    const after = await control.getAttribute('value');
    expect(after, 'Enter committed nothing').not.toBe(before);
    // …and the field shows what the control reports.
    await expect(field.locator('.value')).toHaveText(
      await control.evaluate((element) =>
        (element as unknown as { displayText(): string }).displayText(),
      ),
    );
  });

  test('aria-activedescendant names a row that is actually in the DOM', async ({ page }) => {
    // ⚠ The specific way virtualisation and aria-activedescendant break each other: the cursor
    // moves to a row outside the window, the row is never built, and the announcement silently
    // stops. Checked at both ends of a thirty-seven-row list.
    await open(page, longListStory);
    await settle(page);
    const field = page.locator('#long-list button[role="combobox"]');
    await field.focus();
    await page.keyboard.press('ArrowDown');
    await settle(page);

    for (const key of ['End', 'Home', 'PageDown', 'PageUp', 'ArrowUp']) {
      await page.keyboard.press(key);
      await settle(page);
      const named = await field.getAttribute('aria-activedescendant');
      expect(named, `nothing is active after ${key}`).toBeTruthy();
      const present = await page.locator(`#long-list [role="option"][id="${String(named)}"]`).count();
      expect(present, `${String(named)} is named after ${key} and is not in the DOM`).toBe(1);
    }
  });

  test('a long list builds only its window, and the number comes from the shared planner', async ({ page }) => {
    await open(page, longListStory);
    await settle(page);
    const numbers = await page.locator('#long-list').evaluate((element) => {
      const control = element as unknown as {
        options: readonly unknown[];
        surface?: { builtRowCount: number; expectedCellCount: number; builtWindow: { firstRow: number; lastRow: number } };
      };
      return {
        total: control.options.length,
        built: control.surface?.builtRowCount ?? -1,
        expected: control.surface?.expectedCellCount ?? -1,
        window: control.surface?.builtWindow ?? { firstRow: -1, lastRow: -1 },
      };
    });

    expect(numbers.total).toBeGreaterThan(listboxVisibleRows * 2);
    // Correspondence: the rows built equal what `cellsInWindow` says the window holds.
    expect(numbers.built).toBe(numbers.expected);
    // The ceiling…
    expect(numbers.built).toBeLessThan(numbers.total);
    // …and the anti-vacuity assertion beside it. A list that built nothing would satisfy the line
    // above perfectly, which is exactly how MJXOFF-185's "one preview, not eight" stayed green on
    // zero previews.
    expect(numbers.built).toBeGreaterThan(0);
    expect(numbers.window.lastRow).toBeGreaterThan(numbers.window.firstRow);
  });

  test('the placement is the one `placeFloating` computes in Node from the same inputs', async ({ page }) => {
    await open(page, flipStory);
    await settle(page);
    const recorded = await page.locator('#flipping').evaluate((element) => {
      const control = element as unknown as {
        direction: string;
        surface?: {
          placement?: unknown;
          anchorRect?: unknown;
          naturalSize?: unknown;
          boundaryRect: unknown;
        };
      };
      const surface = control.surface;
      return {
        direction: control.direction,
        placement: surface?.placement as { x: number; y: number; side: string; flipped: boolean } | undefined,
        anchor: surface?.anchorRect as Rect | undefined,
        natural: surface?.naturalSize as { width: number; height: number } | undefined,
        boundary: surface?.boundaryRect as Rect | undefined,
        gap: Number.parseFloat(
          getComputedStyle(element).getPropertyValue('--mjx-floating-gap') || '0',
        ),
      };
    });

    expect(recorded.placement, 'the list recorded no placement').toBeDefined();
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

    const recomputed = placeFloating({
      anchor: recorded.anchor,
      floating: recorded.natural,
      boundary: recorded.boundary,
      side: 'blockEnd',
      align: 'start',
      direction: recorded.direction === 'rtl' ? 'rtl' : 'ltr',
      gap: recorded.gap,
    });
    expect(Math.round(recomputed.x)).toBe(Math.round(recorded.placement.x));
    expect(Math.round(recomputed.y)).toBe(Math.round(recorded.placement.y));
    expect(recomputed.side).toBe(recorded.placement.side);
    // …and the story exists because the list has to flip in it. A gate that only compared two
    // numbers would be green on a list that never left its preferred side.
    expect(recorded.placement.flipped, 'the list did not flip in the story written to make it')
      .toBe(true);
  });
});

// ── the combo box, and the invariant it exists for ───────────────────────────

/** What the field shows, and what the control reports it as. These must be the same string. */
async function fieldAndValue(
  control: Locator,
): Promise<{ shown: string; reported: string; typing: boolean }> {
  return control.evaluate((element) => {
    const combo = element as unknown as { text: string; displayText(): string; typing: boolean };
    return { shown: combo.text, reported: combo.displayText(), typing: combo.typing };
  });
}

test.describe('a combo box', () => {
  /**
   * ⚠ **`finishes` is the half that keeps this suite honest.**
   *
   * Eight of the nine paths end with the field and the value agreeing. One does not, and it is not
   * a defect: Escape while the list is open restores *the text as it was when the list opened*,
   * which is a person's half-typed string and is deliberately not the value. A gate that asserted
   * agreement everywhere would have failed on the one behaviour the ticket asks for by name; a
   * gate that allowed disagreement everywhere would prove nothing. So each path says which it is,
   * and the count of each is asserted below.
   */
  const paths: readonly {
    name: string;
    finishes: 'settled' | 'still typing';
    /** What the field must show when the path finishes still typing. */
    text?: string;
    run: (page: Page, control: Locator) => Promise<void>;
  }[] = [
    {
      name: 'typing an exact label and pressing Enter',
      finishes: 'settled',
      run: async (page, control) => {
        await control.locator('input').fill('Georgia');
        await page.keyboard.press('Enter');
      },
    },
    {
      name: 'typing a prefix, arrowing and pressing Enter',
      finishes: 'settled',
      run: async (page, control) => {
        await control.locator('input').fill('co');
        await page.keyboard.press('ArrowDown');
        await page.keyboard.press('Enter');
      },
    },
    {
      name: 'typing and pressing Escape while the list is open',
      finishes: 'still typing',
      text: 'co',
      run: async (page, control) => {
        await control.locator('input').fill('co');
        await page.keyboard.press('Escape');
      },
    },
    {
      name: 'pressing Escape with the list closed',
      finishes: 'settled',
      run: async (page, control) => {
        await control.locator('input').fill('co');
        await page.keyboard.press('Escape');
        await page.keyboard.press('Escape');
      },
    },
    {
      name: 'typing and pressing Tab',
      finishes: 'settled',
      run: async (page, control) => {
        await control.locator('input').fill('Verdana');
        await page.keyboard.press('Tab');
      },
    },
    {
      name: 'typing a string the list does not carry and pressing Tab',
      finishes: 'settled',
      run: async (page, control) => {
        await control.locator('input').fill('Helvetica');
        await page.keyboard.press('Tab');
      },
    },
    {
      name: 'emptying the field and pressing Enter',
      finishes: 'settled',
      run: async (page, control) => {
        await control.locator('input').fill('');
        await page.keyboard.press('Enter');
      },
    },
    {
      name: 'clicking an option',
      finishes: 'settled',
      run: async (page, control) => {
        await control.locator('input').fill('co');
        await settleShort(page);
        await control.locator('[role="option"]').first().click();
      },
    },
    {
      name: 'typing and clicking outside',
      finishes: 'settled',
      run: async (page, control) => {
        await control.locator('input').fill('Tahoma');
        await settleShort(page);
        await page.locator('body').click({ position: { x: 4, y: 4 } });
      },
    },
  ];

  async function settleShort(page: Page): Promise<void> {
    await settle(page);
  }

  test('exactly one of the nine paths finishes with a person still typing', () => {
    // The anti-vacuity assertion beside the loop below. If `finishes` were allowed to drift to
    // 'still typing' everywhere, every path's assertion would become "the text is whatever it is".
    expect(paths.filter((path) => path.finishes === 'still typing')).toHaveLength(1);
    expect(paths.filter((path) => path.finishes === 'settled')).toHaveLength(8);
  });

  for (const path of paths) {
    test(`the field shows what the control reports, after ${path.name}`, async ({ page }) => {
      await open(page, filterStory);
      await settle(page);
      const control = page.locator('#filtering');
      await control.locator('input').focus();
      await path.run(page, control);
      await settle(page);

      const { shown, reported, typing } = await fieldAndValue(control);
      if (path.finishes === 'settled') {
        expect(typing, `after ${path.name} the control still thinks it is being typed into`).toBe(
          false,
        );
        expect(shown, `after ${path.name} the field and the value disagree`).toBe(reported);
        return;
      }
      // The one path that ends mid-edit: the text is the person's, and the control says so.
      expect(typing, `after ${path.name} the control claims to have settled`).toBe(true);
      expect(shown, `after ${path.name} the field lost what was typed`).toBe(path.text);
    });
  }

  test('typing filters to exactly what the model says it filters to', async ({ page }) => {
    await open(page, filterStory);
    await settle(page);
    const control = page.locator('#filtering');
    await control.locator('input').fill('co');
    await settle(page);

    const options = await control.evaluate((element) =>
      (element as unknown as { options: readonly { value: string; label: string }[] }).options.map(
        (option) => option.label,
      ),
    );
    const expectedLabels = filterOptions(
      options.map((labelText) => ({ value: labelText, label: labelText })),
      'co',
      'contains',
    ).map((option) => option.label);

    const shown = await control.locator('[role="option"]').evaluateAll((rows) =>
      rows.map((row) => row.querySelector('.option-label')?.textContent ?? ''),
    );
    // Correspondence, and the two anti-vacuity bounds: the filter must keep *some* and *not all*.
    expect(shown).toEqual(expectedLabels);
    expect(shown.length).toBeGreaterThan(0);
    expect(shown.length).toBeLessThan(options.length);
  });

  test('the arrows move through the filtered set and not through all of it', async ({ page }) => {
    await open(page, filterStory);
    await settle(page);
    const control = page.locator('#filtering');
    const input = control.locator('input');
    await input.fill('co');
    await settle(page);

    const labels = await control.locator('[role="option"]').evaluateAll((rows) =>
      rows.map((row) => row.querySelector('.option-label')?.textContent ?? ''),
    );
    expect(labels.length).toBeGreaterThan(2);

    // ⚠ Sample the middle, not the end. MJXOFF-184's diagonal test passed with the tolerance
    // removed because it only asserted where the pointer finished.
    const walked: string[] = [];
    for (let index = 0; index < labels.length; index += 1) {
      const named = await input.getAttribute('aria-activedescendant');
      const text = await page
        .locator(`#filtering [role="option"][id="${String(named)}"] .option-label`)
        .textContent();
      walked.push(text ?? '');
      await page.keyboard.press('ArrowDown');
      await settle(page);
    }
    expect(walked).toEqual(labels);
  });

  test('Home and End are caret keys, and the list does not take them', async ({ page }) => {
    await open(page, filterStory);
    await settle(page);
    const control = page.locator('#filtering');
    const input = control.locator('input');
    await input.fill('Cons');
    await settle(page);
    const before = await input.getAttribute('aria-activedescendant');

    await page.keyboard.press('Home');
    await settle(page);
    expect(
      await input.getAttribute('aria-activedescendant'),
      'Home moved the list cursor in a text box',
    ).toBe(before);
    // …and it moved the caret, which is what it is for.
    expect(await input.evaluate((element) => (element as HTMLInputElement).selectionStart)).toBe(0);

    await page.keyboard.press('End');
    await settle(page);
    expect(await input.evaluate((element) => (element as HTMLInputElement).selectionStart)).toBe(4);
  });

  test('Escape restores the text as it was when the list opened — not the value', async ({ page }) => {
    await open(page, filterStory);
    await settle(page);
    const control = page.locator('#filtering');
    const input = control.locator('input');

    // Type, which opens the list; the text at that moment is what Escape must bring back.
    await input.fill('Co');
    await settle(page);
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await settle(page);
    await page.keyboard.press('Escape');
    await settle(page);

    expect(await input.inputValue()).toBe('Co');
    // …and a second Escape, with the list closed, goes back to the value.
    await page.keyboard.press('Escape');
    await settle(page);
    const { shown, reported } = await fieldAndValue(control);
    expect(shown).toBe(reported);
    expect(shown).toBe('Cambria');
  });

  test('a string the list does not carry: refused without allow-custom, kept with it', async ({ page }) => {
    await open(page, customStory);
    await settle(page);

    const refusals = await page.evaluate(() => {
      const seen: unknown[] = [];
      document.addEventListener('mjx-input-invalid', (event) => {
        seen.push((event as CustomEvent).detail);
      });
      (globalThis as { __refused?: unknown[] }).__refused = seen;
      return seen.length;
    });
    expect(refusals).toBe(0);

    const strict = page.locator('#strict');
    await strict.locator('input').fill('Helvetica');
    await page.keyboard.press('Enter');
    await settle(page);
    expect(await strict.getAttribute('value')).toBe('cambria');
    const reported = await page.evaluate(() => (globalThis as { __refused?: unknown[] }).__refused);
    expect(reported).toHaveLength(1);
    // …and the field agrees with the value again rather than keeping the refused string.
    const strictPair = await fieldAndValue(strict);
    expect(strictPair.shown).toBe(strictPair.reported);

    const permissive = page.locator('#permissive');
    await permissive.locator('input').fill('Helvetica');
    await page.keyboard.press('Enter');
    await settle(page);
    expect(await permissive.getAttribute('value')).toBe('Helvetica');
    const permissivePair = await fieldAndValue(permissive);
    expect(permissivePair.shown).toBe('Helvetica');
    expect(permissivePair.reported).toBe('Helvetica');
  });
});

// ── the measure input ────────────────────────────────────────────────────────

test.describe('a measure input', () => {
  test('the same quantity is written correctly in all six units', async ({ page }) => {
    await open(page, unitsStory);
    await settle(page);
    const written = await page.locator('mjx-measure-input input').evaluateAll((inputs) =>
      inputs.map((input) => (input as HTMLInputElement).value),
    );
    expect(written).toEqual([
      formatMeasure(72, 'pt'),
      formatMeasure(72, 'in'),
      formatMeasure(72, 'cm'),
      formatMeasure(72, 'mm'),
      formatMeasure(72, 'px'),
      formatMeasure(72, 'pc'),
    ]);
    // Anti-vacuity: six different strings, not six copies of one.
    expect(new Set(written).size).toBe(6);
  });

  test('a typed measure commits the number the grammar says it means', async ({ page }) => {
    await open(page, rangeStory);
    await settle(page);
    const control = page.locator('#ranged-cm');
    await control.locator('input').fill('2.5 cm');
    await page.keyboard.press('Enter');
    await settle(page);

    const committed = Number.parseFloat((await control.getAttribute('value')) ?? '');
    const wanted = parseMeasure('2.5 cm', 'cm');
    expect(wanted.ok).toBe(true);
    if (wanted.ok) expect(committed).toBeCloseTo(wanted.measure.points, 6);
    // …and the field re-writes it in its own unit.
    expect(await control.locator('input').inputValue()).toBe('2.5 cm');
  });

  test('a comma is accepted in a dot field, and written back the way the field declares', async ({ page }) => {
    await open(page, commaStory);
    await settle(page);
    const control = page.locator('#comma');
    await control.locator('input').fill('1,5 cm');
    await page.keyboard.press('Enter');
    await settle(page);
    const committed = Number.parseFloat((await control.getAttribute('value')) ?? '');
    expect(committed).toBeCloseTo(pointsFrom(1.5, 'cm'), 6);
    expect(await control.locator('input').inputValue()).toBe('1,5 cm');
  });

  test('nonsense: the text stays, the value does not move, and it says so three ways', async ({ page }) => {
    await open(page, nonsenseStory);
    await settle(page);

    await page.evaluate(() => {
      const seen: unknown[] = [];
      document.addEventListener('mjx-input-invalid', (event) => {
        seen.push((event as CustomEvent).detail);
      });
      (globalThis as { __invalid?: unknown[] }).__invalid = seen;
    });

    const control = page.locator('#nonsense');
    const before = await control.getAttribute('value');
    await control.locator('input').fill('banana');
    await page.keyboard.press('Tab');
    await settle(page);

    // 1. The text is untouched.
    expect(await control.locator('input').inputValue()).toBe('banana');
    // 2. The value has not moved — no undo entry, no changed document.
    expect(await control.getAttribute('value')).toBe(before);
    // 3. The three cues `fieldStates.invalid.nonColourCue` declares, each asserted to be *drawn*.
    await expect(control.locator('input')).toHaveAttribute('aria-invalid', 'true');
    await expect(control.locator('[part="invalid-mark"]')).toBeVisible();
    await expect(control.locator('[part="message"]')).toBeVisible();
    await expect(control.locator('[part="message"]')).toContainText('banana');
    // 4. …and the event carried what was refused.
    const reported = await page.evaluate(() => (globalThis as { __invalid?: unknown[] }).__invalid);
    expect(reported).toEqual([{ text: 'banana', failure: 'notANumber', offending: 'banana' }]);
  });

  test('an unknown unit is a different refusal, and names the fragment that defeated it', async ({ page }) => {
    await open(page, nonsenseStory);
    await settle(page);
    const control = page.locator('#wrong-unit');
    await control.locator('input').fill('12 furlongs');
    await page.keyboard.press('Tab');
    await settle(page);
    await expect(control.locator('[part="message"]')).toContainText('furlongs');
    await expect(control.locator('input')).toHaveAttribute('aria-invalid', 'true');
  });

  test('it stays invalid after blur, and Escape is the way out', async ({ page }) => {
    await open(page, nonsenseStory);
    await settle(page);
    const control = page.locator('#nonsense');
    const before = await control.getAttribute('value');
    await control.locator('input').fill('banana');
    await page.locator('body').click({ position: { x: 4, y: 4 } });
    await settle(page);
    // ⚠ Still invalid, still holding the person's text. A field that reverted here would look
    // exactly like one that had committed.
    expect(await control.locator('input').inputValue()).toBe('banana');
    await expect(control.locator('input')).toHaveAttribute('aria-invalid', 'true');

    await control.locator('input').focus();
    await page.keyboard.press('Escape');
    await settle(page);
    await expect(control.locator('input')).toHaveAttribute('aria-invalid', 'false');
    expect(await control.locator('input').inputValue()).toBe(
      formatMeasure(Number.parseFloat(before ?? '0'), 'pt'),
    );
  });

  test('an arrow key steps in the displayed unit and stops at the range’s end', async ({ page }) => {
    await open(page, rangeStory);
    await settle(page);
    const control = page.locator('#ranged-cm');
    const input = control.locator('input');
    await input.focus();

    const start = Number.parseFloat((await control.getAttribute('value')) ?? '');
    await page.keyboard.press('ArrowUp');
    await settle(page);
    const stepped = Number.parseFloat((await control.getAttribute('value')) ?? '');
    // A quarter of a centimetre, not a point.
    expect(stepped - start).toBeCloseTo(pointsFrom(0.25, 'cm'), 6);

    // …and the floor holds.
    const floorControl = page.locator('#ranged-pt');
    await floorControl.locator('input').focus();
    await floorControl.locator('input').fill('1');
    await page.keyboard.press('Enter');
    await page.keyboard.press('ArrowDown');
    await settle(page);
    expect(Number.parseFloat((await floorControl.getAttribute('value')) ?? '')).toBe(1);
  });
});

// ── the slider ───────────────────────────────────────────────────────────────

/** Where the thumb's centre is along the rail, as a fraction. Measured, never read back. */
async function measuredFraction(control: Locator): Promise<number> {
  return control.evaluate((element) => {
    const root = element.shadowRoot;
    const rail = root?.querySelector('.rail');
    const thumb = root?.querySelector('.thumb');
    if (rail === null || rail === undefined || thumb === null || thumb === undefined) return -1;
    const railBox = rail.getBoundingClientRect();
    const thumbBox = thumb.getBoundingClientRect();
    if (railBox.width <= 0) return -1;
    return (thumbBox.left + thumbBox.width / 2 - railBox.left) / railBox.width;
  });
}

test.describe('a slider', () => {
  test('the thumb is where the model says, at five values including both ends', async ({ page }) => {
    await open(page, valuesStory);
    await settle(page);
    const values = [0, 25, 50, 75, 100];
    const measured: number[] = [];
    for (const value of values) {
      const control = page.locator(`mjx-slider[data-at="${String(value)}"]`);
      await expect(control).toHaveCount(1);
      const fraction = await measuredFraction(control);
      measured.push(fraction);
      // Computed in Node from the range, never read back from the custom property the component
      // itself wrote — a component that graded its own homework would agree with itself perfectly.
      expect(fraction, `value ${String(value)}`).toBeCloseTo(sliderFraction(value, 0, 100), 2);
    }
    // ⚠ Anti-vacuity: five *different* positions. A slider that put every thumb at the start would
    // pass the loop above for the value 0 and fail only because the others are checked too — and a
    // `toBeCloseTo` with a wide tolerance could still let it through, which this cannot.
    expect(new Set(measured.map((fraction) => fraction.toFixed(2))).size).toBe(5);
    expect(measured[0]).toBeLessThan(measured[4] ?? 0);
  });

  test('a keyboard step moves by exactly the declared step', async ({ page }) => {
    await open(page, fractionalStory);
    await settle(page);
    const control = page.locator('#spacing');
    await control.locator('[role="slider"]').focus();

    const before = Number.parseFloat((await control.getAttribute('value')) ?? '');
    await page.keyboard.press('ArrowRight');
    await settle(page);
    const after = Number.parseFloat((await control.getAttribute('value')) ?? '');
    expect(after - before).toBeCloseTo(0.05, 10);
    // …and the announcement carries no float tail.
    const announced = await control.locator('[role="slider"]').getAttribute('aria-valuetext');
    expect(announced).toBe('1.2 ×');
    expect(await control.locator('[role="slider"]').getAttribute('aria-valuenow')).toBe('1.2');
  });

  test('Home and End land on the bounds exactly, even off a step boundary', async ({ page }) => {
    await open(page, offBoundaryStory);
    await settle(page);
    const control = page.locator('#off-boundary');
    const track = control.locator('[role="slider"]');
    await track.focus();

    await page.keyboard.press('End');
    await settle(page);
    expect(await control.getAttribute('value')).toBe('10');
    expect(await track.getAttribute('aria-valuenow')).toBe('10');
    expect(await measuredFraction(control)).toBeCloseTo(1, 2);

    await page.keyboard.press('Home');
    await settle(page);
    expect(await control.getAttribute('value')).toBe('0');
    expect(await measuredFraction(control)).toBeCloseTo(0, 2);

    // …and a step past the top is clamped to the top rather than refused.
    for (let index = 0; index < 5; index += 1) await page.keyboard.press('ArrowUp');
    await settle(page);
    expect(await control.getAttribute('value')).toBe('10');
  });

  test('a value off a step boundary is snapped the way the model says', async ({ page }) => {
    await open(page, offBoundaryStory);
    await settle(page);
    const control = page.locator('#off-boundary');
    for (const [written, wanted] of [
      [4, snapToStep(4, { min: 0, max: 10, step: 3 })],
      [9.4, snapToStep(9.4, { min: 0, max: 10, step: 3 })],
      [9.6, snapToStep(9.6, { min: 0, max: 10, step: 3 })],
    ] as const) {
      await control.evaluate((element, value) => {
        element.setAttribute('value', String(value));
      }, written);
      await settle(page);
      const reported = await control.evaluate((element) => (element as unknown as { value: number }).value);
      expect(reported, `${String(written)} should snap to ${String(wanted)}`).toBe(wanted);
      expect(await measuredFraction(control)).toBeCloseTo(sliderFraction(wanted, 0, 10), 2);
    }
  });

  test('under right-to-left the inline arrows mirror and the block ones do not', async ({ page }) => {
    await open(page, sliderRtlStory);
    await settle(page);
    const control = page.locator('#rtl-slider');
    await control.locator('[role="slider"]').focus();

    const before = Number.parseFloat((await control.getAttribute('value')) ?? '');
    await page.keyboard.press('ArrowLeft');
    await settle(page);
    expect(Number.parseFloat((await control.getAttribute('value')) ?? '')).toBe(before + 10);

    await page.keyboard.press('ArrowDown');
    await settle(page);
    expect(Number.parseFloat((await control.getAttribute('value')) ?? '')).toBe(before);
  });

  test('the track announces its whole range, not just its value', async ({ page }) => {
    await open(page, { title: slider, name: 'With Ticks' });
    await settle(page);
    const track = page.locator('#zoom [role="slider"]');
    await expect(track).toHaveAttribute('aria-valuemin', '10');
    await expect(track).toHaveAttribute('aria-valuemax', '400');
    await expect(track).toHaveAttribute('aria-valuenow', '100');
    await expect(track).toHaveAttribute('aria-valuetext', '100 %');
  });

  test('a slider with no native disabled still announces that it is unavailable', async ({ page }) => {
    await open(page, { title: slider, name: 'Unavailable And Disabled' });
    await settle(page);
    const tracks = page.locator('mjx-slider [role="slider"]');
    await expect(tracks).toHaveCount(2);
    // Explained: reachable, announced, carrying its reason.
    await expect(tracks.nth(0)).toHaveAttribute('aria-disabled', 'true');
    expect(await tracks.nth(0).evaluate((element) => (element as HTMLElement).tabIndex)).toBe(0);
    // ⚠ Hard-disabled, on an element with no native `disabled` property. The widened
    // `applyAvailability` must announce it, and the component must take it out of the tab order.
    await expect(tracks.nth(1)).toHaveAttribute('aria-disabled', 'true');
    expect(await tracks.nth(1).evaluate((element) => (element as HTMLElement).tabIndex)).toBe(-1);
  });
});

// ── the segmented control ────────────────────────────────────────────────────

test.describe('a segmented control', () => {
  test('the arrows move and choose, wrapping at both ends', async ({ page }) => {
    await open(page, alignmentStory);
    await settle(page);
    const control = page.locator('#alignment');
    await control.locator('button[aria-checked="true"]').focus();
    expect(await control.getAttribute('value')).toBe('left');

    const walked: (string | null)[] = [];
    for (let index = 0; index < 5; index += 1) {
      await page.keyboard.press('ArrowRight');
      await settle(page);
      walked.push(await control.getAttribute('value'));
    }
    // Four members, so the fifth press is back at the first — and the middle of the walk is
    // asserted rather than only its end.
    expect(walked).toEqual(['center', 'right', 'justify', 'left', 'center']);
  });

  test('exactly one member is checked, and it is the one the value names', async ({ page }) => {
    await open(page, alignmentStory);
    await settle(page);
    const checked = page.locator('#alignment button[aria-checked="true"]');
    await expect(checked).toHaveCount(1);
    await expect(checked).toHaveText('Left');
  });

  test('an unavailable member is reachable and refused', async ({ page }) => {
    await open(page, unavailableSegmentStory);
    await settle(page);
    const control = page.locator('#with-unavailable');
    await control.locator('button[aria-checked="true"]').focus();
    await page.keyboard.press('End');
    await settle(page);
    // The tab stop moved there so it can be read…
    const focused = await deepActive(page);
    expect(focused.role).toBe('radio');
    // …and nothing was chosen.
    expect(await control.getAttribute('value')).toBe('left');
    await expect(control.locator('button').last()).toHaveAttribute('aria-disabled', 'true');
  });
});

// ── the label ────────────────────────────────────────────────────────────────

test.describe('a label', () => {
  test('its text becomes the control’s accessible name', async ({ page }) => {
    await open(page, namingStory);
    await settle(page);
    expect(await page.locator('#named-size').getAttribute('label')).toBe('Font size');
    await expect(page.locator('#named-size input')).toHaveAttribute('aria-label', 'Font size');
    await expect(page.locator('#named-spacing [role="slider"]')).toHaveAttribute(
      'aria-label',
      'Line spacing',
    );
  });

  test('it never overwrites a name the control already declared', async ({ page }) => {
    await open(page, namingStory);
    await settle(page);
    // The caption says one thing and the control says another; the control wins. Author a default
    // only where nothing exists.
    expect(await page.locator('#already-named').getAttribute('label')).toBe('Keep with next');
  });

  test('a press on the label focuses the control it names', async ({ page }) => {
    await open(page, namingStory);
    await settle(page);
    await page.locator('mjx-label[for="named-size"]').click();
    await settle(page);
    const active = await deepActive(page);
    expect(active.name).toBe('Font size');
  });

  test('a required field announces the word as well as drawing the mark', async ({ page }) => {
    await open(page, { title: label, name: 'Required And Hinted' });
    await settle(page);
    const required = page.locator('mjx-label[required]').first();
    await expect(required.locator('.required')).toHaveAttribute('aria-hidden', 'true');
    await expect(required.locator('.visually-hidden')).toHaveText(/required/);
    // …and the label with no `required` draws neither.
    const optional = page.locator('mjx-label').nth(1);
    expect(await optional.locator('.required').evaluate((element) => (element as HTMLElement).hidden))
      .toBe(true);
  });
});

// ── the row plan the list borrows ────────────────────────────────────────────

test('a sectioned list draws one heading per section, and they are one row tall', async ({ page }) => {
  await open(page, longListStory);
  await settle(page);
  const heights = await page.locator('#long-list .section').evaluateAll((headings) =>
    headings.map((heading) => heading.getBoundingClientRect().height),
  );
  const rows = await page.locator('#long-list [role="option"]').evaluateAll((options) =>
    options.map((option) => option.getBoundingClientRect().height),
  );
  expect(rows.length).toBeGreaterThan(0);
  // Every row the same height — the arithmetic the virtualiser divides by.
  expect(new Set(rows.map((height) => height.toFixed(1))).size).toBe(1);
  for (const height of heights) {
    expect(height, 'a heading is not one row tall').toBeCloseTo(rows[0] ?? 0, 1);
  }
  // …and the plan says how many rows there should be in total.
  const total = await page.locator('#long-list').evaluate((element) => {
    const control = element as unknown as { options: readonly { category?: string }[] };
    return control.options.map((option) => option.category ?? '');
  });
  const plan = galleryRowPlan(total, 1, true);
  const headings = plan.filter((row) => row.kind === 'heading').length;
  expect(headings).toBe(2);
  expect(cellsInWindow(plan, { firstRow: 0, lastRow: plan.length })).toBe(total.length);
});
