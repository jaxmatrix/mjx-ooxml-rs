import { expect, test, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import { accessibleHitTargetMinimum } from '../../src/foundations/density.ts';
import {
  archetypeStoryTitle,
  archetypeTags,
  componentStateMatrix,
  controlArchetypes,
  controlSizes,
  effectiveStatePaint,
  largeControlWidthUnits,
  stateCellAttribute,
  statesMatrixStoryName,
  type ControlArchetype,
  type ControlState,
  type Paint,
} from '../../src/controls/control-states.ts';

/**
 * The four ribbon archetypes, measured in a browser.
 *
 * `tests/controls.test.ts` proves the state *model* is internally distinct. This proves the four
 * things a model cannot say anything about, and every one of them is a failure that looks correct
 * in a screenshot:
 *
 * * **The paint the browser actually computed is distinct, pairwise, in both schemes** — and
 *   **equals the model**, which is what stops the two gates from agreeing with each other about a
 *   rendering neither of them looked at.
 * * **A forced state is the real state.** The catalogue shows `hover` and `active` through a
 *   `data-state` attribute, because a static story cannot hover. The rules are shared by
 *   construction; this drives a real pointer over a real control and requires the two to compute
 *   identically anyway, because a structural guarantee nobody has watched hold is a guarantee about
 *   a file.
 * * **The behaviour is the platform's.** Enter and Space are pressed through the browser's own
 *   input path — never as synthetic events, which prove a handler exists and prove nothing about
 *   whether the platform would ever call it — and a control that cannot be used must produce no
 *   event by any route.
 * * **The foundations reached the shadow roots.** This is the one MJXOFF-182 was handed explicitly:
 *   `installFoundations(shadowRoot)` was guarded by a single assertion in the whole suite, because
 *   nothing before this child rendered in a shadow root that needed it. Four separate consequences
 *   of it are read back below, on all four components, including the *order* it was installed in.
 *
 * ⚠ No colour, length or duration is written in this file. Every expectation comes from the
 * generated tokens or from `src/controls/control-states.ts`.
 */

const schemes: readonly ColorScheme[] = ['light', 'dark'];

/**
 * Wait for a hover or press transition to finish before reading a colour.
 *
 * Every control wears `.mjx-motion-surface-settle`, so a fill does not change — it *travels*, over
 * `--duration-transition`. Reading it too early returns a colour that is on the way to the answer
 * and equal to nothing: MJXOFF-182 first saw this as `rgba(239, 233, 220, 0.133)` where a fully
 * opaque `--theme-border-subtle` was expected, and as a region that appeared to still be lit after
 * the pointer had left it.
 *
 * The wait is **derived from the token**, twice over plus a frame's grace, so a re-seed that slows
 * the platform's motion does not silently make this suite flaky. A fixed number here would have
 * been exactly the literal the whole child is written against.
 */
const transitionMilliseconds = Number.parseFloat(tokens.duration.transition);

async function settle(page: Page): Promise<void> {
  await page.waitForTimeout(transitionMilliseconds * 2 + 100);
}

/** Open a story by title and name, failing with the name rather than with `undefined`. */
async function open(
  page: Page,
  title: string,
  name: string,
  options: { theme?: string; containerPreset?: string } = {},
): Promise<void> {
  const story = builtStories().find((entry) => entry.title === title && entry.name === name);
  expect(story, `${title} · ${name} is missing from the catalogue`).toBeDefined();
  if (story === undefined) return;
  await openStory(page, story.id, options);
  // The controls upgrade asynchronously; a computed style read before the upgrade would report the
  // element's pre-render box and have nothing to do with the component.
  await page.waitForFunction(() => customElements.get('mjx-button') !== undefined);
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
}

/** Open one archetype's states matrix. */
async function openMatrix(
  page: Page,
  archetype: ControlArchetype,
  theme: ColorScheme,
): Promise<void> {
  await open(page, archetypeStoryTitle[archetype], statesMatrixStoryName, { theme });
}

/** `#2e9e63` → `rgb(46, 158, 99)`, which is what `getComputedStyle` returns. */
function expectedColor(paint: Paint | undefined, scheme: ColorScheme): string {
  if (paint === undefined || paint === 'transparent') return 'rgba(0, 0, 0, 0)';
  const hex = tokens.theme[scheme][paint];
  const number = Number.parseInt(hex.slice(1, 7), 16);
  return `rgb(${String((number >> 16) & 0xff)}, ${String((number >> 8) & 0xff)}, ${String(number & 0xff)})`;
}

/**
 * The in-page reader: every property that distinguishes one state from another, off one element.
 *
 * Injected as a string rather than imported, because it runs in the browser. It reads the same
 * seven things `resolvedStateFingerprint` models — fill, edge, edge style, text colour, weight,
 * opacity, inset ring — plus the outline, which is how `focus` differs from `rest` and which no
 * rule in this child writes: it comes from the foundations.
 */
const readPaint = `(element) => {
  const style = getComputedStyle(element);
  return {
    background: style.backgroundColor,
    borderColor: style.borderTopColor,
    borderStyle: style.borderTopStyle,
    color: style.color,
    weight: style.fontWeight,
    opacity: style.opacity,
    shadow: style.boxShadow,
    outline: style.outlineStyle + ' ' + style.outlineColor + ' ' + style.outlineWidth,
    fontSize: style.fontSize,
    minBlock: style.minBlockSize,
    minInline: style.minInlineSize,
  };
}`;

interface Paints {
  background: string;
  borderColor: string;
  borderStyle: string;
  color: string;
  weight: string;
  opacity: string;
  shadow: string;
  outline: string;
  fontSize: string;
  minBlock: string;
  minInline: string;
}

function fingerprint(paint: Paints): string {
  return [
    paint.background,
    paint.borderColor,
    paint.borderStyle,
    paint.color,
    paint.weight,
    paint.opacity,
    paint.shadow,
    paint.outline,
  ].join(' | ');
}

/** Every matrix cell: its state, and the paint of the control inside it. */
async function readCells(page: Page): Promise<{ state: ControlState; paint: Paints }[]> {
  return page.evaluate(
    ([attribute, tags, reader]) => {
      const read = new Function(`return ${reader}`)() as (element: Element) => unknown;
      return [...document.querySelectorAll(`[${attribute}]`)].map((cell) => {
        const host = cell.querySelector(tags);
        const control = host?.shadowRoot?.querySelector('.control');
        if (control === null || control === undefined) {
          throw new Error(`the cell for '${cell.getAttribute(attribute) ?? '?'}' has no control`);
        }
        return {
          state: cell.getAttribute(attribute) as never,
          paint: read(control) as never,
        };
      });
    },
    [stateCellAttribute, Object.values(archetypeTags).join(', '), readPaint] as const,
  );
}

/** The deepest focused element, described in terms a test can assert on. */
const activeDescriptor = `() => {
  let element = document.activeElement;
  while (element && element.shadowRoot && element.shadowRoot.activeElement) {
    element = element.shadowRoot.activeElement;
  }
  const root = element ? element.getRootNode() : null;
  const host = root instanceof ShadowRoot ? root.host : null;
  const cell = host ? host.closest('[data-state-cell]') : null;
  return {
    tag: element ? element.tagName.toLowerCase() : null,
    hostTag: host ? host.tagName.toLowerCase() : null,
    region: element instanceof HTMLElement ? (element.dataset.region ?? null) : null,
    cell: cell ? cell.getAttribute('data-state-cell') : null,
    name: element ? (element.textContent ?? '').trim() : '',
    ariaDisabled: element ? element.getAttribute('aria-disabled') : null,
    ariaPressed: element ? element.getAttribute('aria-pressed') : null,
  };
}`;

interface ActiveDescriptor {
  tag: string | null;
  hostTag: string | null;
  region: string | null;
  cell: string | null;
  name: string;
  ariaDisabled: string | null;
  ariaPressed: string | null;
}

async function focused(page: Page): Promise<ActiveDescriptor> {
  return page.evaluate(
    (source) => (new Function(`return ${source}`)() as () => unknown)() as never,
    activeDescriptor,
  );
}

/** Press Tab until the predicate holds, or give up and say how far it got. */
async function tabUntil(
  page: Page,
  matches: (descriptor: ActiveDescriptor) => boolean,
  limit = 24,
): Promise<{ reached: boolean; visited: ActiveDescriptor[] }> {
  const visited: ActiveDescriptor[] = [];
  for (let step = 0; step < limit; step += 1) {
    await page.keyboard.press('Tab');
    const descriptor = await focused(page);
    visited.push(descriptor);
    if (matches(descriptor)) return { reached: true, visited };
  }
  return { reached: false, visited };
}

/** Start recording the three control events. Re-installed after every navigation. */
async function recordEvents(page: Page): Promise<void> {
  await page.evaluate(() => {
    const store: { type: string; detail: unknown }[] = [];
    (globalThis as unknown as { mjxEvents: typeof store }).mjxEvents = store;
    for (const type of ['mjx-activate', 'mjx-change', 'mjx-menu-request']) {
      document.addEventListener(type, (event) => {
        store.push({ type: event.type, detail: (event as CustomEvent).detail });
      });
    }
  });
}

async function events(page: Page): Promise<{ type: string; detail: unknown }[]> {
  return page.evaluate(
    () => (globalThis as unknown as { mjxEvents: { type: string; detail: unknown }[] }).mjxEvents,
  );
}

async function clearEvents(page: Page): Promise<void> {
  await page.evaluate(() => {
    (globalThis as unknown as { mjxEvents: unknown[] }).mjxEvents.length = 0;
  });
}

/**
 * The bounding box of the **control** in one matrix cell — not of the cell.
 *
 * A cell is a grid of the control and its caption, so its centre is as likely to be on the caption
 * as on the thing under test. A pointer test that pressed the caption would find no event and
 * report it as a broken button.
 */
async function cellBox(
  page: Page,
  state: ControlState,
): Promise<{ x: number; y: number; width: number; height: number }> {
  const selector = `[${stateCellAttribute}="${state}"] :is(${Object.values(archetypeTags).join(', ')})`;
  const box = await page.locator(selector).first().boundingBox();
  expect(box, `the '${state}' cell has no control with a box`).not.toBeNull();
  return box ?? { x: 0, y: 0, width: 0, height: 0 };
}

// ── the shadow-root foundations, made load-bearing ───────────────────────────

test.describe('the foundations reach every control’s shadow root', () => {
  test('all four adopt them, adopt them first, and show four consequences of having done so', async ({
    page,
  }) => {
    await open(page, 'Controls/Density', 'Both Densities');

    const report = await page.evaluate(
      ([tags, reader]) => {
        const read = new Function(`return ${reader}`)() as (element: Element) => unknown;
        return tags.map((tag) => {
          const host = document.querySelector(tag);
          const root = host?.shadowRoot ?? null;
          const sheets = root === null ? [] : [...root.adoptedStyleSheets];
          const texts = sheets.map((sheet) =>
            [...sheet.cssRules].map((rule) => rule.cssText).join('\n'),
          );
          const control = root?.querySelector('.control') ?? null;
          return {
            tag,
            found: host !== null,
            sheetCount: sheets.length,
            foundationsIndex: texts.findIndex((text) => text.includes('.mjx-hit-target')),
            componentIndex: texts.findIndex((text) => text.includes('data-pressed')),
            paint: control === null ? null : (read(control) as never),
          };
        });
      },
      [Object.values(archetypeTags), readPaint] as const,
    );

    expect(report.length).toBe(controlArchetypes.length);

    // What the `control` type role resolves to on this page, read from an element the foundations
    // are known to style — so the expectation is the cascade's own answer rather than a number.
    const expectedFontSize = await page.evaluate(() => {
      const probe = document.createElement('span');
      probe.className = 'mjx-type-control';
      document.body.append(probe);
      const size = getComputedStyle(probe).fontSize;
      probe.remove();
      return size;
    });

    for (const entry of report) {
      expect(entry.found, `${entry.tag} is not in the story`).toBe(true);
      const paint = entry.paint as Paints | null;
      expect(paint, `${entry.tag} has no .control in its shadow root`).not.toBeNull();
      if (paint === null) continue;

      // 1. The sheet is there at all.
      expect(
        entry.foundationsIndex,
        `${entry.tag}'s shadow root never adopted the foundations. A shadow root inherits custom ` +
          'properties but not rules, so this component has no type scale, no hit-target floor, no ' +
          'motion and no focus ring — and its stories would still look plausible.',
      ).toBeGreaterThanOrEqual(0);

      // 2. …and the component's own sheet is adopted AFTER it, which is what lets a state rule
      //    win a tie against `.mjx-type-control`. Both score (0,0,0); only order decides.
      expect(entry.componentIndex, `${entry.tag} adopted no component sheet`).toBeGreaterThanOrEqual(
        0,
      );
      expect(
        entry.componentIndex,
        `${entry.tag} adopted its own rules BEFORE the foundations. Every state rule and every ` +
          'foundations rule scores (0,0,0), so this order is the whole cascade: with it reversed, ' +
          "`.mjx-type-control`'s medium weight defeats the pressed state's bold and two states " +
          'render alike.',
      ).toBeGreaterThan(entry.foundationsIndex);

      // 3. The hit-target floor, which only `.mjx-hit-target` in that sheet provides.
      expect(
        Number.parseFloat(paint.minBlock),
        `${entry.tag}: min-block-size is ${paint.minBlock}`,
      ).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
      expect(Number.parseFloat(paint.minInline)).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);

      // 4. The type role, which only `.mjx-type-control` in that sheet provides.
      expect(paint.fontSize, `${entry.tag}: the control type role did not reach it`).toBe(
        expectedFontSize,
      );
    }
  });

  test('and the focus ring on a control is the foundations’ ring, not one of its own', async ({
    page,
  }) => {
    // The fourth consequence, and the one MJXOFF-181 wrote the whole focus foundation to guarantee:
    // *a button must not define its own focus ring.* Nothing in `src/controls/` writes `outline`,
    // so a ring appearing here is the adopted sheet's, and no ring appearing means the adoption
    // did not happen.
    await openMatrix(page, 'button', 'light');
    await page.locator('body').click({ position: { x: 1, y: 1 } });
    const { reached } = await tabUntil(page, (descriptor) => descriptor.cell === 'focus');
    expect(reached, 'Tab never reached the focus cell').toBe(true);

    const outline = await page.evaluate(() => {
      let element = document.activeElement;
      while (element?.shadowRoot?.activeElement != null) element = element.shadowRoot.activeElement;
      const style = element === null ? null : getComputedStyle(element);
      return style === null
        ? null
        : { style: style.outlineStyle, width: style.outlineWidth, color: style.outlineColor };
    });
    expect(outline).not.toBeNull();
    expect(outline?.style, 'a keyboard-focused control has no ring').not.toBe('none');
    expect(Number.parseFloat(outline?.width ?? '0')).toBeGreaterThan(0);

    // The colour is the foundations' — `--theme-accent-pressed` — rather than a UA default.
    expect(outline?.color).toBe(expectedColor('accentPressed', 'light'));
  });
});

// ── the pairwise gate ────────────────────────────────────────────────────────

test.describe('pairwise state distinctness, on what the browser computed', () => {
  for (const archetype of controlArchetypes) {
    for (const scheme of schemes) {
      test(`${archetype} · ${scheme}: every pair of states differs`, async ({ page }) => {
        await openMatrix(page, archetype, scheme);
        const expectedStates = componentStateMatrix[archetype];

        const cells = await readCells(page);
        expect(
          cells.map((cell) => cell.state),
          'the matrix story does not show the states the component claims. If this found nothing ' +
            'at all, the story stopped marking its cells and the gate was about to sweep an empty ' +
            'list — which is why the count is asserted rather than the emptiness tolerated.',
        ).toEqual([...expectedStates]);

        // `focus` cannot be forced: it is produced by pressing Tab, through the browser's own
        // input path, so `:focus-visible` makes its own judgement rather than being asserted into
        // existence.
        await page.locator('body').click({ position: { x: 1, y: 1 } });
        const { reached } = await tabUntil(page, (descriptor) => descriptor.cell === 'focus');
        expect(reached, 'Tab never reached the focus cell').toBe(true);
        const focusedCells = await readCells(page);

        const measured = new Map<string, ControlState>();
        for (const [index, cell] of cells.entries()) {
          const paint = cell.state === 'focus' ? focusedCells[index]?.paint : cell.paint;
          expect(paint, `no paint for '${cell.state}'`).toBeDefined();
          if (paint === undefined) continue;
          const print = fingerprint(paint);
          const previous = measured.get(print);
          expect(
            previous,
            `${archetype} renders '${cell.state}' and '${String(previous)}' identically in ` +
              `${scheme}:\n  ${print}\n` +
              'Two states that compute alike pass every "the state exists" check and are ' +
              'invisible to an auditor.',
          ).toBeUndefined();
          measured.set(print, cell.state);
        }
        expect(measured.size).toBe(expectedStates.length);
      });

      test(`${archetype} · ${scheme}: and what it computed is what the model says`, async ({
        page,
      }) => {
        // The two gates must not merely agree with each other. This is the join: the fill and the
        // edge the browser produced, compared with `effectiveStatePaint` resolved through the
        // generated tokens for this scheme.
        await openMatrix(page, archetype, scheme);
        const cells = await readCells(page);
        for (const cell of cells) {
          const model = effectiveStatePaint(cell.state);
          expect(
            cell.paint.background,
            `${archetype} · ${cell.state} · ${scheme}: fill is ${cell.paint.background}, model ` +
              `says ${String(model.background)}`,
          ).toBe(expectedColor(model.background, scheme));
          expect(
            cell.paint.borderColor,
            `${archetype} · ${cell.state} · ${scheme}: edge is ${cell.paint.borderColor}`,
          ).toBe(expectedColor(model.borderColor, scheme));
          expect(cell.paint.borderStyle).toBe(model.borderStyle);
          expect(cell.paint.color).toBe(expectedColor(model.text, scheme));
          expect(Number.parseFloat(cell.paint.opacity)).toBeCloseTo(model.opacity, 3);
          // The weight is the second thing a held state says, and it is the one that goes when
          // the component's sheet is adopted *before* the foundations: `.mjx-type-control`'s
          // medium then wins the (0,0,0) tie and the pressed states quietly stop being bold. The
          // fills still differ, so the pairwise gate would not notice — this is what does.
          expect(
            cell.paint.weight,
            `${archetype} · ${cell.state} · ${scheme}: font-weight is ${cell.paint.weight}, model ` +
              `says ${model.weight}`,
          ).toBe(String(tokens.fontWeight[model.weight]));
        }
      });
    }
  }
});

test.describe('a forced state is the real state', () => {
  test('hovering a resting button computes exactly what force-state="hover" does', async ({
    page,
  }) => {
    await openMatrix(page, 'button', 'light');
    const forced = (await readCells(page)).find((cell) => cell.state === 'hover');
    expect(forced).toBeDefined();

    const rest = await cellBox(page, 'rest');
    await page.mouse.move(rest.x + rest.width / 2, rest.y + rest.height / 2);
    await settle(page);
    const hovered = (await readCells(page)).find((cell) => cell.state === 'rest');
    expect(hovered).toBeDefined();

    expect(
      hovered === undefined ? '' : fingerprint(hovered.paint),
      'a real pointer hover and the catalogue’s forced hover do not compute the same thing, so ' +
        'the states matrix is showing a mock of the component rather than the component.',
    ).toBe(forced === undefined ? '' : fingerprint(forced.paint));
  });

  test('and holding the pointer down computes exactly what force-state="active" does', async ({
    page,
  }) => {
    await openMatrix(page, 'button', 'light');
    const forced = (await readCells(page)).find((cell) => cell.state === 'active');
    const rest = await cellBox(page, 'rest');
    await page.mouse.move(rest.x + rest.width / 2, rest.y + rest.height / 2);
    await page.mouse.down();
    await settle(page);
    const held = (await readCells(page)).find((cell) => cell.state === 'rest');
    await page.mouse.up();

    expect(held === undefined ? '' : fingerprint(held.paint)).toBe(
      forced === undefined ? '' : fingerprint(forced.paint),
    );
  });

  test('a pressed toggle under the pointer takes the onHover paint, which is the cascade order', async ({
    page,
  }) => {
    await openMatrix(page, 'toggleButton', 'light');
    const forced = (await readCells(page)).find((cell) => cell.state === 'onHover');
    const on = await cellBox(page, 'on');
    await page.mouse.move(on.x + on.width / 2, on.y + on.height / 2);
    await settle(page);
    const hovered = (await readCells(page)).find((cell) => cell.state === 'on');

    expect(
      hovered === undefined ? '' : fingerprint(hovered.paint),
      'hovering a pressed toggle produced neither the pressed-and-hovered paint nor anything ' +
        'like it. The rules all score (0,0,0), so this is an assertion about `controlStateCascade` ' +
        'and it means a rule has been inserted in the wrong place.',
    ).toBe(forced === undefined ? '' : fingerprint(forced.paint));
  });
});

// ── behaviour ────────────────────────────────────────────────────────────────

test.describe('a button behaves like a button', () => {
  test('Enter and Space each activate it, exactly once, through the platform', async ({ page }) => {
    await openMatrix(page, 'button', 'light');
    await recordEvents(page);
    await page.locator('body').click({ position: { x: 1, y: 1 } });

    const { reached } = await tabUntil(page, (descriptor) => descriptor.cell === 'rest');
    expect(reached, 'Tab never reached the resting button').toBe(true);

    await page.keyboard.press('Enter');
    expect((await events(page)).map((entry) => entry.type)).toEqual(['mjx-activate']);

    await clearEvents(page);
    await page.keyboard.press('Space');
    expect(
      (await events(page)).map((entry) => entry.type),
      'Space did not activate the button. Both keys operate a button, and a component that only ' +
        'answers Enter is one a keyboard user meets as broken.',
    ).toEqual(['mjx-activate']);
  });

  test('a disabled button is out of the tab order and inert to the pointer', async ({ page }) => {
    await openMatrix(page, 'button', 'light');
    await recordEvents(page);
    await page.locator('body').click({ position: { x: 1, y: 1 } });

    const { visited } = await tabUntil(page, () => false, 10);
    expect(
      visited.map((descriptor) => descriptor.cell),
      'Tab reached the disabled control. A hard-disabled button is out of the tab order — that is ' +
        'the platform’s own behaviour, and reaching it means the native attribute is not being set.',
    ).not.toContain('disabled');

    const box = await cellBox(page, 'disabled');
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    expect(await events(page)).toEqual([]);
  });

  test('an unavailable button is reachable, explained, and still refuses to act', async ({
    page,
  }) => {
    await openMatrix(page, 'button', 'light');
    await recordEvents(page);
    await page.locator('body').click({ position: { x: 1, y: 1 } });

    const { reached } = await tabUntil(page, (descriptor) => descriptor.cell === 'unavailable');
    expect(
      reached,
      'Tab never reached the unavailable control. The whole point of this state rather than a ' +
        'plain disabled one is that a person can reach it and read why it cannot be used.',
    ).toBe(true);

    const descriptor = await focused(page);
    expect(descriptor.ariaDisabled, 'it is not announced as unavailable').toBe('true');

    // The explanation is announced, not merely present: the described-by element is resolved and
    // its text read, which is what a screen reader would do.
    const description = await page.evaluate(() => {
      let element = document.activeElement;
      while (element?.shadowRoot?.activeElement != null) element = element.shadowRoot.activeElement;
      const id = element?.getAttribute('aria-describedby') ?? '';
      const root = element?.getRootNode();
      const target = root instanceof ShadowRoot ? root.getElementById(id) : null;
      return { text: (target?.textContent ?? '').trim(), title: element?.getAttribute('title') ?? '' };
    });
    expect(description.text, 'the unavailable control carries no explanation').not.toBe('');
    // …and the pointer user gets the same sentence, which is the other half of "explained".
    expect(description.title).toBe(description.text);

    await page.keyboard.press('Enter');
    await page.keyboard.press('Space');
    const box = await cellBox(page, 'unavailable');
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    expect(
      await events(page),
      'an unavailable control was activated. It is `aria-disabled`, not `disabled`, so the ' +
        'platform still delivers the click and the refusal has to be written down.',
    ).toEqual([]);
  });

  test('a large button’s long label wraps to two lines and is clamped there', async ({ page }) => {
    await open(page, archetypeStoryTitle.button, 'Two Line Labels');
    const measured = await page.evaluate(() => {
      return [...document.querySelectorAll('mjx-button')].map((host) => {
        const control = host.shadowRoot?.querySelector('.control') as HTMLElement | null;
        const label = host.shadowRoot?.querySelector('.label') as HTMLElement | null;
        const style = label === null ? null : getComputedStyle(label);
        return {
          label: host.getAttribute('label') ?? '',
          lineHeight: style === null ? 0 : Number.parseFloat(style.lineHeight),
          height: label?.getBoundingClientRect().height ?? 0,
          scrollHeight: label?.scrollHeight ?? 0,
          clientHeight: label?.clientHeight ?? 0,
          width: control?.getBoundingClientRect().width ?? 0,
        };
      });
    });
    expect(measured.length).toBe(3);

    const spacing = await page.evaluate(() => {
      const probe = document.createElement('div');
      probe.style.inlineSize = 'var(--spacing)';
      document.body.append(probe);
      const value = Number.parseFloat(getComputedStyle(probe).inlineSize);
      probe.remove();
      return value;
    });
    const maximum = spacing * largeControlWidthUnits.max;

    const [single, wrapped, clamped] = measured;
    expect(single).toBeDefined();
    expect(wrapped).toBeDefined();
    expect(clamped).toBeDefined();
    if (single === undefined || wrapped === undefined || clamped === undefined) return;

    // One word: one line. Two words that do not fit: two lines, and the button no wider than the
    // size's bound — a label that "wrapped" by making the button three words wide has not wrapped.
    expect(Math.round(single.height / single.lineHeight)).toBe(1);
    expect(Math.round(wrapped.height / wrapped.lineHeight)).toBe(controlSizes.large.labelLines);
    expect(wrapped.width).toBeLessThanOrEqual(maximum + 1);

    // Four words: still two lines, and demonstrably cut off rather than merely short.
    expect(Math.round(clamped.height / clamped.lineHeight)).toBe(controlSizes.large.labelLines);
    expect(
      clamped.scrollHeight,
      'the longest label was not clamped — it fits, so this specimen proves nothing',
    ).toBeGreaterThan(clamped.clientHeight);
  });
});

test.describe('a toggle holds a state, and says so', () => {
  test('aria-pressed moves with every activation, and the event follows it', async ({ page }) => {
    await openMatrix(page, 'toggleButton', 'light');
    await recordEvents(page);
    await page.locator('body').click({ position: { x: 1, y: 1 } });

    const { reached } = await tabUntil(page, (descriptor) => descriptor.cell === 'rest');
    expect(reached).toBe(true);
    expect((await focused(page)).ariaPressed, 'an off toggle must still announce itself').toBe(
      'false',
    );

    await page.keyboard.press('Enter');
    expect((await focused(page)).ariaPressed).toBe('true');
    expect(await events(page)).toEqual([{ type: 'mjx-change', detail: { pressed: 'true' } }]);

    await clearEvents(page);
    await page.keyboard.press('Space');
    expect((await focused(page)).ariaPressed).toBe('false');
    expect(await events(page)).toEqual([{ type: 'mjx-change', detail: { pressed: 'false' } }]);
  });

  test('a mixed toggle announces “mixed” and resolves to pressed when it is used', async ({
    page,
  }) => {
    await openMatrix(page, 'toggleButton', 'light');
    await recordEvents(page);

    const before = await page.evaluate(() => {
      const cell = document.querySelector('[data-state-cell="mixed"] mjx-toggle-button');
      return cell?.shadowRoot?.querySelector('.control')?.getAttribute('aria-pressed') ?? null;
    });
    expect(before, 'the indeterminate state is not announced as mixed').toBe('mixed');

    const box = await cellBox(page, 'mixed');
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    expect(await events(page)).toEqual([{ type: 'mjx-change', detail: { pressed: 'true' } }]);
  });

  test('a disabled toggle cannot be moved by any route', async ({ page }) => {
    await openMatrix(page, 'toggleButton', 'light');
    await recordEvents(page);
    for (const state of ['disabled', 'unavailable'] as const) {
      const box = await cellBox(page, state);
      await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    }
    expect(await events(page)).toEqual([]);
  });
});

// ── the split button ─────────────────────────────────────────────────────────

test.describe('a split button is two controls, and they are independent', () => {
  test('two regions, two names, two tab stops', async ({ page }) => {
    await open(page, archetypeStoryTitle.splitButton, 'The Two Regions');

    const regions = await page.evaluate(() => {
      const host = document.querySelector('mjx-split-button');
      const buttons = [...(host?.shadowRoot?.querySelectorAll('button') ?? [])];
      return buttons.map((button) => ({
        region: button.dataset['region'] ?? '',
        name: (button.textContent ?? '').trim(),
        haspopup: button.getAttribute('aria-haspopup'),
        expanded: button.getAttribute('aria-expanded'),
      }));
    });
    expect(regions.map((entry) => entry.region)).toEqual(['primary', 'menu']);
    expect(
      regions[0]?.name,
      'the two regions share one accessible name, which is two commands with one name',
    ).not.toBe(regions[1]?.name);
    expect(regions[0]?.name).not.toBe('');
    expect(regions[1]?.name).not.toBe('');
    expect(regions[1]?.haspopup).toBe('menu');
    expect(regions[1]?.expanded).toBe('false');

    await page.locator('body').click({ position: { x: 1, y: 1 } });
    const primary = await tabUntil(page, (descriptor) => descriptor.region === 'primary');
    expect(primary.reached, 'Tab never reached the primary action').toBe(true);
    await page.keyboard.press('Tab');
    expect(
      (await focused(page)).region,
      'the next tab stop after the primary action is not the menu arrow. A split button whose ' +
        'arrow a keyboard cannot reach has a menu nobody can open.',
    ).toBe('menu');
  });

  test('hovering one region leaves the other at rest', async ({ page }) => {
    await open(page, archetypeStoryTitle.splitButton, 'The Two Regions');

    const boxes = await page.evaluate(() => {
      const host = document.querySelector('mjx-split-button');
      const buttons = [...(host?.shadowRoot?.querySelectorAll('button') ?? [])];
      return buttons.map((button) => {
        const rect = button.getBoundingClientRect();
        return { region: button.dataset['region'] ?? '', x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
      });
    });

    const read = async (): Promise<Record<string, string>> =>
      page.evaluate((reader) => {
        const readPaintFn = new Function(`return ${reader}`)() as (element: Element) => Paints;
        const host = document.querySelector('mjx-split-button');
        const result: Record<string, string> = {};
        for (const button of host?.shadowRoot?.querySelectorAll('button') ?? []) {
          const paint = readPaintFn(button);
          result[button.dataset['region'] ?? ''] =
            `${paint.background} | ${paint.borderColor} | ${paint.shadow}`;
        }
        return result;
      }, readPaint);

    await page.mouse.move(0, 0);
    await settle(page);
    const resting = await read();

    for (const box of boxes) {
      await page.mouse.move(box.x, box.y);
      await settle(page);
      const hovered = await read();
      const other = box.region === 'primary' ? 'menu' : 'primary';
      expect(
        hovered[box.region],
        `hovering the ${box.region} region did not change it`,
      ).not.toBe(resting[box.region]);
      expect(
        hovered[other],
        `hovering the ${box.region} region also lit the ${other} region. That is what a :hover ` +
          'written on the host does, and it means the control never tells a person which half ' +
          'they are about to press.',
      ).toBe(resting[other]);
    }
    await page.mouse.move(0, 0);
  });

  test('the pointer hits whichever region it is over, and they do different things', async ({
    page,
  }) => {
    await open(page, archetypeStoryTitle.splitButton, 'The Two Regions');
    await recordEvents(page);

    const boxes = await page.evaluate(() => {
      const host = document.querySelector('mjx-split-button');
      return [...(host?.shadowRoot?.querySelectorAll('button') ?? [])].map((button) => {
        const rect = button.getBoundingClientRect();
        return {
          region: button.dataset['region'] ?? '',
          x: rect.x + rect.width / 2,
          y: rect.y + rect.height / 2,
        };
      });
    });

    for (const box of boxes) {
      await clearEvents(page);
      await page.mouse.click(box.x, box.y);
      const fired = await events(page);
      expect(fired.length, `pressing the ${box.region} region fired ${String(fired.length)} events`).toBe(1);
      expect(fired[0]?.type).toBe(box.region === 'primary' ? 'mjx-activate' : 'mjx-menu-request');
      expect(fired[0]?.detail).toEqual({ region: box.region });
    }
  });

  test('Arrow Down opens the menu and never fires the action', async ({ page }) => {
    await open(page, archetypeStoryTitle.splitButton, 'The Two Regions');
    await recordEvents(page);
    await page.locator('body').click({ position: { x: 1, y: 1 } });

    const primary = await tabUntil(page, (descriptor) => descriptor.region === 'primary');
    expect(primary.reached).toBe(true);

    await clearEvents(page);
    await page.keyboard.press('ArrowDown');
    expect(
      await events(page),
      'Arrow Down on the primary region did not ask for the menu, or asked for it *and* fired ' +
        'the action. Somebody who meant to see the paste options and instead pasted has an edit ' +
        'to undo.',
    ).toEqual([{ type: 'mjx-menu-request', detail: { region: 'primary' } }]);

    await clearEvents(page);
    await page.keyboard.press('Enter');
    expect(await events(page)).toEqual([{ type: 'mjx-activate', detail: { region: 'primary' } }]);

    await page.keyboard.press('Tab');
    await clearEvents(page);
    await page.keyboard.press('Enter');
    expect(
      await events(page),
      'Enter on the menu arrow fired the action. The arrow is not the action.',
    ).toEqual([{ type: 'mjx-menu-request', detail: { region: 'menu' } }]);

    await clearEvents(page);
    await page.keyboard.press('Space');
    expect(await events(page)).toEqual([{ type: 'mjx-menu-request', detail: { region: 'menu' } }]);
  });

  test('aria-expanded follows the host, never the event', async ({ page }) => {
    await open(page, archetypeStoryTitle.splitButton, 'The Two Regions');
    const expanded = await page.evaluate(() =>
      [...document.querySelectorAll('mjx-split-button')].map((host) => ({
        declared: host.hasAttribute('expanded'),
        announced: host.shadowRoot
          ?.querySelector('[data-region="menu"]')
          ?.getAttribute('aria-expanded'),
      })),
    );
    expect(expanded.length).toBeGreaterThan(1);
    for (const entry of expanded) {
      expect(entry.announced).toBe(entry.declared ? 'true' : 'false');
    }
    expect(
      expanded.some((entry) => entry.declared),
      'no specimen declares `expanded`, so the assertion above is satisfied by a control that ' +
        'always says false',
    ).toBe(true);
  });
});

// ── the dialog launcher ──────────────────────────────────────────────────────

test.describe('a dialog launcher names its dialog', () => {
  test('every launcher in the catalogue has a name, and says it opens a dialog', async ({
    page,
  }) => {
    for (const name of [statesMatrixStoryName, 'In A Group Header']) {
      await open(page, archetypeStoryTitle.dialogLauncher, name);
      const launchers = await page.evaluate(() =>
        [...document.querySelectorAll('mjx-dialog-launcher')].map((host) => {
          const button = host.shadowRoot?.querySelector('button');
          return {
            name: (button?.textContent ?? '').trim(),
            haspopup: button?.getAttribute('aria-haspopup') ?? null,
            expanded: button?.getAttribute('aria-expanded') ?? null,
          };
        }),
      );
      expect(launchers.length, `${name} shows no launcher`).toBeGreaterThan(0);
      for (const launcher of launchers) {
        expect(
          launcher.name,
          `${name}: a launcher has no accessible name. "Dialog launcher" tells a screen-reader ` +
            'user nothing about which dialog.',
        ).not.toBe('');
        expect(launcher.haspopup).toBe('dialog');
        // A dialog is not a disclosure: claiming aria-expanded would announce a state it does not
        // have.
        expect(launcher.expanded).toBeNull();
      }
    }
  });
});

// ── hit targets ──────────────────────────────────────────────────────────────

test.describe('every control clears the accessible target size', () => {
  for (const preset of ['desktop', 'phone'] as const) {
    test(`in both density modes, at the ${preset} container`, async ({ page }) => {
      await open(page, 'Controls/Density', 'Both Densities', { containerPreset: preset });

      const measured = await page.evaluate(() =>
        [...document.querySelectorAll('[data-density-probe]')].flatMap((probe) => {
          const mode = probe.getAttribute('data-density-probe') ?? '';
          const cluster = probe.querySelector('div');
          const gap = cluster === null ? 0 : Number.parseFloat(getComputedStyle(cluster).columnGap);
          return [...probe.querySelectorAll('mjx-button, mjx-toggle-button, mjx-split-button, mjx-dialog-launcher')].flatMap(
            (host) =>
              [...(host.shadowRoot?.querySelectorAll('.control') ?? [])].map((control) => {
                const style = getComputedStyle(control);
                const rect = control.getBoundingClientRect();
                return {
                  mode,
                  gap,
                  tag: host.tagName.toLowerCase(),
                  region: (control as HTMLElement).dataset['region'] ?? '',
                  minBlock: Number.parseFloat(style.minBlockSize),
                  minInline: Number.parseFloat(style.minInlineSize),
                  width: rect.width,
                  height: rect.height,
                };
              }),
          );
        }),
      );

      expect(measured.length).toBeGreaterThan(0);
      const modes = new Set(measured.map((entry) => entry.mode));
      expect(modes, 'both density modes must be measured').toEqual(
        new Set(['comfortable', 'compact']),
      );

      for (const entry of measured) {
        const where = `${entry.mode} · ${entry.tag}${entry.region === '' ? '' : ` (${entry.region})`}`;
        expect(entry.minBlock, `${where}: min-block-size`).toBeGreaterThanOrEqual(
          accessibleHitTargetMinimum,
        );
        expect(entry.minInline, `${where}: min-inline-size`).toBeGreaterThanOrEqual(
          accessibleHitTargetMinimum,
        );
        // The painted box, not just the declared minimum — a control can declare a floor and be
        // squashed by its layout.
        expect(entry.height, `${where}: painted height`).toBeGreaterThanOrEqual(
          accessibleHitTargetMinimum,
        );
        expect(entry.width, `${where}: painted width`).toBeGreaterThanOrEqual(
          accessibleHitTargetMinimum,
        );
      }

      // …and compact genuinely is compact, or the floor assertion above is satisfied by a density
      // mode that changes nothing.
      const comfortable = measured.find((entry) => entry.mode === 'comfortable');
      const compact = measured.find((entry) => entry.mode === 'compact');
      expect(compact?.gap ?? 0).toBeLessThan(comfortable?.gap ?? 0);
      expect(compact?.minBlock ?? 0).toBeLessThan(comfortable?.minBlock ?? 0);
    });
  }
});
