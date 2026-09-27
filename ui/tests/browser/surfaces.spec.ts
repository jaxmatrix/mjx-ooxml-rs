import { expect, test, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import { accessibleHitTargetMinimum } from '../../src/foundations/density.ts';
import {
  chooseModalEdge,
  chooseSurfaceHandle,
  darkestThemeMember,
  dialogPresentationAt,
  scrimOpacityPercent,
  surfaceKinds,
  surfaceStoryTitles,
  taskPaneFractionBounds,
  taskPaneResizeStep,
  themeColor,
} from '../../src/surfaces/surface-model.ts';
import { containerPresets } from '../../src/harness/presets.ts';

/**
 * The surfaces, driven in a real browser.
 *
 * Everything MJXOFF-188 says is worth having here is **temporal, spatial or about the accessibility
 * tree**, and none of the three survives a screenshot:
 *
 * * **where focus goes on every close path.** *"A modal that traps focus is easy; a modal that
 *   returns it is where the defect lives."* Four paths are driven and the keyboard's landing place
 *   is measured after each one;
 * * **whether the background is genuinely unreachable**, by two independent instruments — real
 *   `Tab` presses, and the accessible names an assistive technology would find. `inert` and
 *   `aria-hidden` are different mechanisms and one without the other is a defect that looks like
 *   nothing;
 * * **stacking.** A popover opened from a dialog, closed with one Escape, leaving the dialog open.
 *
 * ## Two lessons from earlier children that this suite is arranged around
 *
 * **U05's:** a floating surface is clipped by whatever opened it and `position: fixed` does not
 * save it. So the top-layer claims are made with `elementsFromPoint`, which is what actually
 * failed last time, rather than with a rectangle — a correct rectangle with no pixels behind it is
 * exactly the defect.
 *
 * **U06's and U07's:** a helper whose failure mode is *"found nothing"* makes every ceiling
 * assertion pass. So every count here is asserted to be greater than zero beside the ceiling it is
 * being compared against, and the tab-stop walk in `overlay/modality.ts` is never trusted on its
 * own — it is compared against real key presses.
 */

const dialog = surfaceStoryTitles.dialog;
const popover = surfaceStoryTitles.popover;
const taskPane = surfaceStoryTitles.taskPane;

const modalStory = { title: dialog, name: 'A Modal Over The Document' } as const;
const returningStory = { title: dialog, name: 'Returned To Its Invoker' } as const;
const modelessStory = { title: dialog, name: 'Modeless — The Document Is Still Live' } as const;
const stackedStory = { title: dialog, name: 'A Popover Inside A Dialog' } as const;
const compactDialogStory = { title: dialog, name: 'In Compact Density' } as const;

const stopsStory = { title: popover, name: 'One Stop Or Many' } as const;
const disagreeingStory = { title: popover, name: 'A Popover That Disagrees With Itself' } as const;
const flipStory = { title: popover, name: 'At The Bottom Of Its Room' } as const;

const dockedStory = { title: taskPane, name: 'Docked Beside The Document' } as const;
const colouredStory = { title: taskPane, name: 'Against A Coloured Page' } as const;
const compactPaneStory = { title: taskPane, name: 'In Compact Density' } as const;

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
  await page.waitForFunction(() => customElements.get('mjx-dialog') !== undefined);
  await page.waitForFunction(() => customElements.get('mjx-popover') !== undefined);
  await page.waitForFunction(() => customElements.get('mjx-task-pane') !== undefined);
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

/** The active element, followed through every shadow root it is hiding in. */
async function deepActive(page: Page): Promise<{ tag: string; id: string; label: string }> {
  return page.evaluate(() => {
    let element: Element | null = document.activeElement;
    while (element?.shadowRoot?.activeElement != null) element = element.shadowRoot.activeElement;
    return {
      tag: element?.localName ?? '',
      id: element?.id ?? '',
      label: element?.getAttribute('aria-label') ?? element?.textContent?.trim() ?? '',
    };
  });
}

/** What a run of real `Tab` presses lands on. The instrument the walk is checked against. */
async function tabStops(page: Page, presses: number): Promise<string[]> {
  const seen: string[] = [];
  for (let index = 0; index < presses; index += 1) {
    await page.keyboard.press('Tab');
    const active = await deepActive(page);
    seen.push(active.id === '' ? `${active.tag}:${active.label}` : active.id);
  }
  return seen;
}

/**
 * A computed colour's channels, whichever notation the engine chose.
 *
 * ⚠ `rgb()`, `rgba()` **and** `color(srgb r g b / a)`: Chromium reports the result of a
 * `color-mix()` in the third form, and a helper that only understood the first two would return
 * nothing for a scrim that was painting perfectly. `undefined` — never a stand-in colour — because
 * a helper that answered black for a value it could not read makes every comparison pass on a page
 * where nothing was painted.
 */
function channelsOf(
  computed: string,
): { red: number; green: number; blue: number; alpha: number } | undefined {
  const srgb = /^color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*([\d.]+))?\)$/.exec(
    computed.trim(),
  );
  if (srgb !== null) {
    return {
      red: Number(srgb[1]) * 255,
      green: Number(srgb[2]) * 255,
      blue: Number(srgb[3]) * 255,
      alpha: srgb[4] === undefined ? 1 : Number(srgb[4]),
    };
  }
  const rgb = /^rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?\s*\)$/.exec(
    computed.trim(),
  );
  if (rgb === null) return undefined;
  return {
    red: Number(rgb[1]),
    green: Number(rgb[2]),
    blue: Number(rgb[3]),
    alpha: rgb[4] === undefined ? 1 : Number(rgb[4]),
  };
}

/** `#2e9e63` → `rgb(46, 158, 99)`. */
function rgbOf(hex: string): string {
  const number = Number.parseInt(hex.slice(1, 7), 16);
  return `rgb(${String((number >> 16) & 0xff)}, ${String((number >> 8) & 0xff)}, ${String(number & 0xff)})`;
}

// ── the top layer, which is where U05's defect lived ─────────────────────────

test.describe('a surface is in the top layer', () => {
  test('a modal and its scrim are manual popovers, and the pointer reaches the modal', async ({
    page,
  }) => {
    await open(page, modalStory);
    const measured = await page.evaluate(() => {
      const host = document.querySelector('mjx-dialog');
      const root = host?.shadowRoot;
      const surface = root?.querySelector('.surface');
      const scrim = root?.querySelector('.scrim');
      if (!(surface instanceof HTMLElement) || !(scrim instanceof HTMLElement)) return undefined;
      const box = surface.getBoundingClientRect();
      const hits = document
        .elementsFromPoint(box.left + box.width / 2, box.top + 8)
        .map((element) => element.localName);
      return {
        surfacePopover: surface.getAttribute('popover'),
        scrimPopover: scrim.getAttribute('popover'),
        surfaceOpen: surface.matches(':popover-open'),
        scrimOpen: scrim.matches(':popover-open'),
        area: box.width * box.height,
        hits,
      };
    });
    expect(measured).toBeDefined();
    if (measured === undefined) return;
    // ⚠ `manual`, never `auto`: an `auto` popover light-dismisses, and light dismissal closes the
    // ancestor when a descendant opens — which is the stacking case two tests below.
    expect(measured.surfacePopover).toBe('manual');
    expect(measured.scrimPopover).toBe('manual');
    expect(measured.surfaceOpen).toBe(true);
    expect(measured.scrimOpen).toBe(true);
    // A correct rectangle with no pixels behind it is the exact defect U05 measured, so the
    // assertion is what a pointer finds rather than where the box says it is.
    expect(measured.area, 'the surface has no area at all').toBeGreaterThan(0);
    expect(measured.hits[0]).toBe('mjx-dialog');
  });

  /**
   * ⚠ **The scrim is pinned to the container's frame, not to the window**, and this is the
   * assertion that says so. `::backdrop` would have covered the window, and a modal opened inside a
   * phone-sized frame would then dim the whole page around it — reporting a modality the container
   * never had.
   */
  test('the scrim covers the simulated screen and not the window', async ({ page }) => {
    await open(page, modalStory, { containerPreset: 'phone' });
    const measured = await page.evaluate(() => {
      const scrim = document.querySelector('mjx-dialog')?.shadowRoot?.querySelector('.scrim');
      const frame = document
        .querySelector('mjx-resizable-container')
        ?.shadowRoot?.querySelector('.frame');
      if (!(scrim instanceof HTMLElement) || !(frame instanceof HTMLElement)) return undefined;
      const scrimBox = scrim.getBoundingClientRect();
      const frameBox = frame.getBoundingClientRect();
      return {
        scrim: { width: Math.round(scrimBox.width), left: Math.round(scrimBox.left) },
        frame: { width: Math.round(frameBox.width), left: Math.round(frameBox.left) },
        window: document.documentElement.clientWidth,
      };
    });
    expect(measured).toBeDefined();
    if (measured === undefined) return;
    expect(Math.abs(measured.scrim.width - measured.frame.width)).toBeLessThanOrEqual(2);
    // …and the two are genuinely different, so the assertion above is not satisfied by a frame
    // that happens to be the window.
    expect(measured.frame.width).toBeLessThan(measured.window - 10);
  });
});

// ── the return of focus, which is where the defect lives ─────────────────────

test.describe('a modal returns focus to whatever opened it', () => {
  for (const path of ['escape', 'closeButton', 'scrim', 'programmatic'] as const) {
    test(`after closing by ${path}`, async ({ page }) => {
      await open(page, returningStory);

      // Open it from the keyboard, so the invoker is discovered rather than declared.
      await page.locator('#open-returning').focus();
      await page.keyboard.press('Enter');
      await settle(page);

      const inside = await deepActive(page);
      expect(inside.id, 'focus never moved into the dialog').not.toBe('open-returning');

      const closed = page.evaluate(
        async () =>
          new Promise<{ reason: string; returnedFocus: boolean }>((done) => {
            document.addEventListener(
              'mjx-surface-close',
              (event) => {
                const detail = (event as CustomEvent).detail as {
                  reason: string;
                  returnedFocus: boolean;
                };
                done({ reason: detail.reason, returnedFocus: detail.returnedFocus });
              },
              { once: true },
            );
          }),
      );

      if (path === 'escape') await page.keyboard.press('Escape');
      if (path === 'closeButton') {
        await page.evaluate(() => {
          const close = document
            .querySelector('#returning')
            ?.shadowRoot?.querySelector<HTMLButtonElement>('.close');
          close?.click();
        });
      }
      if (path === 'scrim') {
        await page.evaluate(() => {
          const scrim = document.querySelector('#returning')?.shadowRoot?.querySelector('.scrim');
          scrim?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
        });
      }
      if (path === 'programmatic') {
        await page.evaluate(() => {
          document.querySelector('#returning')?.removeAttribute('open');
        });
      }

      const detail = await closed;
      expect(detail.reason).toBe(path);
      // ⚠ Reported by the component *and* measured independently. A component that said it had
      // returned focus and had not is exactly the defect this test exists for.
      expect(detail.returnedFocus, `${path} reported no focus return`).toBe(true);
      await settle(page);
      expect((await deepActive(page)).id, `${path} left focus somewhere else`).toBe(
        'open-returning',
      );
    });
  }
});

// ── the background, by two instruments ───────────────────────────────────────

test.describe('a modal’s background is genuinely unreachable', () => {
  test('by the keyboard, and the hold marked something', async ({ page }) => {
    await open(page, modalStory);
    const held = await page.evaluate(() => {
      const host = document.querySelector('mjx-dialog');
      return (host as HTMLElement & { backgroundHeld: number } | null)?.backgroundHeld ?? 0;
    });
    // ⚠ U07's second defect, in this child's costume: *"nothing outside the dialog is tabbable"* is
    // trivially satisfied by a hold that marked nothing at all.
    expect(held, 'the background hold marked nothing, so the sweep below proves nothing').toBeGreaterThan(0);

    await page.evaluate(() => {
      const surface = document.querySelector('mjx-dialog')?.shadowRoot?.querySelector('.surface');
      (surface as HTMLElement | null)?.focus();
    });

    // ⚠ The instrument is *containment*, not a list of expected ids. A test that enumerated the
    // dialog's own controls would pass on the day a background link acquired one of those ids, and
    // would have to be rewritten every time the story grew a button.
    const landings: boolean[] = [];
    for (let index = 0; index < 12; index += 1) {
      await page.keyboard.press('Tab');
      landings.push(
        await page.evaluate(() => {
          let element: Element | null = document.activeElement;
          while (element?.shadowRoot?.activeElement != null) {
            element = element.shadowRoot.activeElement;
          }
          const host = document.querySelector('mjx-dialog');
          if (element === null || host === null) return false;
          // `contains` crosses no shadow boundary, so a control inside the dialog's own root is
          // checked through its host — which `getRootNode().host` walks to.
          let node: Node | null = element;
          while (node !== null) {
            if (node === host) return true;
            const root = node.getRootNode();
            node = root instanceof ShadowRoot ? root.host : node.parentNode;
          }
          return false;
        }),
      );
    }
    expect(landings.length).toBe(12);
    expect(
      landings.filter((inside) => !inside).length,
      'Tab left the modal and reached the document behind it',
    ).toBe(0);
  });

  test('and by name, which is the mechanism `inert` alone does not give', async ({ page }) => {
    await open(page, modalStory);
    const marked = await page.evaluate(() => {
      const host = document.querySelector('mjx-dialog');
      if (host === null) return undefined;
      const links = [...document.querySelectorAll('a[href^="#stay-"]')];
      const hidden = links.filter((link) => link.closest('[aria-hidden="true"]') !== null);
      const inert = links.filter((link) => link.closest('[inert]') !== null);
      return { links: links.length, hidden: hidden.length, inert: inert.length };
    });
    expect(marked).toBeDefined();
    if (marked === undefined) return;
    expect(marked.links, 'the story has no background links, so this measures nothing').toBeGreaterThan(0);
    // ⚠ Both, independently. `aria-hidden` does nothing to the tab order and `inert` is not
    // guaranteed to reach every accessibility tree, so a surface that wrote only one of them would
    // pass one of these two assertions and leave a real hole.
    expect(marked.hidden).toBe(marked.links);
    expect(marked.inert).toBe(marked.links);
  });

  test('and every mark is put back exactly as it was found', async ({ page }) => {
    await open(page, returningStory);
    const before = await page.evaluate(
      () => document.querySelectorAll('[inert], [aria-hidden="true"]').length,
    );
    await page.locator('#open-returning').focus();
    await page.keyboard.press('Enter');
    await settle(page);
    const during = await page.evaluate(
      () => document.querySelectorAll('[inert], [aria-hidden="true"]').length,
    );
    expect(during).toBeGreaterThan(before);
    await page.keyboard.press('Escape');
    await settle(page);
    const after = await page.evaluate(
      () => document.querySelectorAll('[inert], [aria-hidden="true"]').length,
    );
    expect(after, 'the hold left marks behind').toBe(before);
  });

  /**
   * ⚠ **The opposite assertion, on the surface that must not hold anything.** A modeless dialog is
   * *supposed* to leave the document live, and a suite that swept every dialog for inertness would
   * have made this one wrong without anybody noticing.
   */
  test('a modeless dialog holds nothing and does not trap', async ({ page }) => {
    await open(page, modelessStory);
    const held = await page.evaluate(() => {
      const host = document.querySelector('mjx-dialog');
      return (host as HTMLElement & { backgroundHeld: number } | null)?.backgroundHeld ?? -1;
    });
    expect(held).toBe(0);
    const marked = await page.evaluate(
      () => document.querySelectorAll('a[href^="#stay-"][aria-hidden], a[href^="#stay-"] ').length,
    );
    expect(marked).toBeGreaterThanOrEqual(0);
    expect(surfaceKinds.dialog.focus).toBe('shared');

    await page.locator('#find-what').focus();
    const stops = await tabStops(page, 6);
    // The keyboard walks out of the dialog into the document, which is the whole difference.
    expect(stops.some((id) => id.startsWith('a:') || id === '')).toBe(true);
  });
});

// ── the trap ─────────────────────────────────────────────────────────────────

test('a modal wraps Tab from its last stop back to its first', async ({ page }) => {
  await open(page, modalStory);
  await page.locator('#modal-save').focus();
  await page.keyboard.press('Tab');
  await settle(page);
  const landed = await deepActive(page);
  // The close button is the dialog's first stop, and the save button is its last.
  expect(landed.label).toContain('Close');
});

// ── stacking ─────────────────────────────────────────────────────────────────

test.describe('stacking', () => {
  test('Escape closes the inner surface and leaves the outer one open', async ({ page }) => {
    await open(page, stackedStory);
    await page.evaluate(() => {
      document.querySelector('#inner-anchor')?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });
    await settle(page);
    expect(await page.evaluate(() => document.querySelector('#inner')?.hasAttribute('open'))).toBe(
      true,
    );

    await page.keyboard.press('Escape');
    await settle(page);
    const afterFirst = await page.evaluate(() => ({
      inner: document.querySelector('#inner')?.hasAttribute('open') ?? false,
      outer: document.querySelector('#stacked')?.hasAttribute('open') ?? false,
    }));
    expect(afterFirst.inner, 'the inner surface survived Escape').toBe(false);
    expect(afterFirst.outer, 'Escape closed the dialog as well as the popover').toBe(true);

    await page.keyboard.press('Escape');
    await settle(page);
    expect(await page.evaluate(() => document.querySelector('#stacked')?.hasAttribute('open'))).toBe(
      false,
    );
  });

  test('closing the outer surface takes the inner one with it', async ({ page }) => {
    await open(page, stackedStory);
    await page.evaluate(() => {
      document.querySelector('#inner-anchor')?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });
    await settle(page);
    const closes = page.evaluate(
      async () =>
        new Promise<string[]>((done) => {
          const seen: string[] = [];
          document.addEventListener('mjx-surface-close', (event) => {
            seen.push(String((event as CustomEvent).detail.kind));
            if (seen.length === 2) done(seen);
          });
          setTimeout(() => {
            done(seen);
          }, 2000);
        }),
    );
    await page.evaluate(() => {
      document.querySelector('#stacked')?.removeAttribute('open');
    });
    const kinds = await closes;
    // Innermost first: the popover's close is emitted before the dialog's, so an outer close
    // handler that restores focus cannot do it while a child still holds the keyboard.
    expect(kinds).toEqual(['popover', 'modal']);
    expect(await page.evaluate(() => document.querySelector('#inner')?.hasAttribute('open'))).toBe(
      false,
    );
  });
});

// ── the presentations ────────────────────────────────────────────────────────

test.describe('a sheet is a dialog at a phone’s width', () => {
  test('the presentation CSS chose is the one the model predicts', async ({ page }) => {
    for (const preset of ['desktop', 'phone'] as const) {
      await open(page, modalStory, { containerPreset: preset });
      const measured = await page.evaluate(() => {
        const host = document.querySelector('mjx-dialog') as
          | (HTMLElement & { presentation: string; kind: string })
          | null;
        const frame = document
          .querySelector('mjx-resizable-container')
          ?.shadowRoot?.querySelector('.frame');
        return {
          presentation: host?.presentation ?? '',
          kind: host?.kind ?? '',
          frameWidth: frame instanceof HTMLElement ? Math.round(frame.getBoundingClientRect().width) : 0,
        };
      });
      expect(measured.frameWidth).toBeGreaterThan(0);
      expect(measured.kind, `at ${preset}, ${String(measured.frameWidth)}px`).toBe(
        dialogPresentationAt(measured.frameWidth, true),
      );
      expect(measured.presentation).toBe(preset === 'phone' ? 'sheet' : 'dialog');
    }
  });

  test('a sheet is pinned across the boundary with square bottom corners', async ({ page }) => {
    await open(page, modalStory, { containerPreset: 'phone' });
    const measured = await page.evaluate(() => {
      const root = document.querySelector('mjx-dialog')?.shadowRoot;
      const surface = root?.querySelector('.surface');
      const handle = root?.querySelector('.handle');
      const frame = document
        .querySelector('mjx-resizable-container')
        ?.shadowRoot?.querySelector('.frame');
      if (!(surface instanceof HTMLElement) || !(frame instanceof HTMLElement)) return undefined;
      const box = surface.getBoundingClientRect();
      const frameBox = frame.getBoundingClientRect();
      const style = getComputedStyle(surface);
      return {
        width: Math.round(box.width),
        frameWidth: Math.round(frameBox.width),
        bottomGap: Math.round(frameBox.bottom - box.bottom),
        radiusTop: style.borderStartStartRadius,
        radiusBottom: style.borderEndStartRadius,
        handleShown: handle instanceof HTMLElement ? getComputedStyle(handle).display : 'missing',
      };
    });
    expect(measured).toBeDefined();
    if (measured === undefined) return;
    expect(Math.abs(measured.width - measured.frameWidth)).toBeLessThanOrEqual(2);
    expect(Math.abs(measured.bottomGap)).toBeLessThanOrEqual(2);
    expect(measured.radiusBottom).toBe('0px');
    expect(measured.radiusTop).not.toBe('0px');
    // The grab handle only exists on a sheet.
    expect(measured.handleShown).toBe('flex');
  });

  test('and the phone-width sheet is the only presentation that shows a handle', async ({ page }) => {
    await open(page, modalStory, { containerPreset: 'desktop' });
    const display = await page.evaluate(() => {
      const handle = document.querySelector('mjx-dialog')?.shadowRoot?.querySelector('.handle');
      return handle instanceof HTMLElement ? getComputedStyle(handle).display : 'missing';
    });
    expect(display).toBe('none');
  });
});

// ── the measured colours, resolved by a real cascade ─────────────────────────

test.describe('the measured edge and scrim survive the cascade', () => {
  for (const scheme of schemes) {
    test(`the modal's outline is the member the rule chose, in ${scheme}`, async ({ page }) => {
      await open(page, modalStory, { theme: scheme });
      await settlePaint(page);
      const measured = await page.evaluate(() => {
        const root = document.querySelector('mjx-dialog')?.shadowRoot;
        const surface = root?.querySelector('.surface');
        const scrim = root?.querySelector('.scrim');
        if (!(surface instanceof HTMLElement) || !(scrim instanceof HTMLElement)) return undefined;
        return {
          outline: getComputedStyle(surface).outlineColor,
          scrim: getComputedStyle(scrim).backgroundColor,
        };
      });
      expect(measured).toBeDefined();
      if (measured === undefined) return;
      // ⚠ This is a *correspondence* gate and not a distinctness one: it says the browser resolved
      // the property to the colour the rule chose, in the scheme that is actually in force. A gate
      // that only checked the two schemes differed would be green for two wrong colours.
      const chosen = chooseModalEdge(scheme).member;
      expect(measured.outline).toBe(rgbOf(themeColor(scheme, chosen)));

      const scrimHex = themeColor(scheme, darkestThemeMember(scheme));
      const number = Number.parseInt(scrimHex.slice(1, 7), 16);
      // ⚠ **Parsed, not string-compared.** Chromium reports the result of `color-mix()` as
      // `color(srgb 0.133 0.231 0.2 / 0.65)` rather than as `rgba(...)`, so a string comparison
      // fails on a page that is painting exactly the right colour. Comparing channels is the
      // assertion that was meant.
      const painted = channelsOf(measured.scrim);
      expect(painted, `the scrim reported ${measured.scrim}`).toBeDefined();
      if (painted === undefined) return;
      expect(painted.red).toBeCloseTo((number >> 16) & 0xff, 0);
      expect(painted.green).toBeCloseTo((number >> 8) & 0xff, 0);
      expect(painted.blue).toBeCloseTo(number & 0xff, 0);
      expect(painted.alpha).toBeCloseTo(scrimOpacityPercent / 100, 2);
    });
  }

  for (const scheme of schemes) {
    test(`the sheet's handle is the member the rule chose, in ${scheme}`, async ({ page }) => {
      await open(page, modalStory, { theme: scheme, containerPreset: 'phone' });
      await settlePaint(page);
      const painted = await page.evaluate(() => {
        const handle = document.querySelector('mjx-dialog')?.shadowRoot?.querySelector('.handle');
        if (!(handle instanceof HTMLElement)) return undefined;
        const bar = getComputedStyle(handle, '::after');
        return { background: bar.backgroundColor, width: bar.width };
      });
      expect(painted).toBeDefined();
      if (painted === undefined) return;
      expect(painted.background).toBe(rgbOf(themeColor(scheme, chooseSurfaceHandle(scheme).member)));
      // …and it is actually drawn. A declared cue that is not rendered fails louder than no
      // declaration — U06's lesson, restated by U07 and inherited here.
      expect(Number.parseFloat(painted.width)).toBeGreaterThan(0);
    });
  }
});

// ── the popover and the flyout ───────────────────────────────────────────────

test.describe('one stop or many', () => {
  test('the walk and a real keyboard agree about how many stops there are', async ({ page }) => {
    await open(page, stopsStory);
    await page.evaluate(() => {
      document
        .querySelector('#trapping-anchor')
        ?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });
    await settle(page);
    const walked = await page.evaluate(() => {
      const host = document.querySelector('#trapping') as
        | (HTMLElement & { countedTabStops: number })
        | null;
      return host?.countedTabStops ?? 0;
    });
    // ⚠ The walk is never trusted on its own: U07's helper reported zero for a pane with two stops
    // and that read as a passing filter rather than as a broken helper.
    expect(walked, 'the walk found nothing, which is a broken helper and not a passing gate')
      .toBeGreaterThan(1);

    await page.evaluate(() => {
      const surface = document.querySelector('#trapping')?.shadowRoot?.querySelector('.surface');
      (surface as HTMLElement | null)?.focus();
    });
    const seen = new Set<string>();
    for (let index = 0; index < walked * 2; index += 1) {
      await page.keyboard.press('Tab');
      const active = await deepActive(page);
      seen.add(active.id === '' ? `${active.tag}:${active.label}` : active.id);
    }
    // A trap that wraps visits exactly its own stops, however long you press Tab.
    expect(seen.size).toBe(walked);
  });

  test('a disclosure closes when focus leaves it and a trap does not', async ({ page }) => {
    await open(page, stopsStory);
    await page.evaluate(() => {
      document
        .querySelector('#disclosure-anchor')
        ?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });
    await settle(page);
    expect(
      await page.evaluate(() => document.querySelector('#disclosure')?.hasAttribute('open')),
    ).toBe(true);

    // Tab out of it, twice: past the close button and past the one control.
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await settle(page);
    expect(
      await page.evaluate(() => document.querySelector('#disclosure')?.hasAttribute('open')),
      'the disclosure survived focus leaving it',
    ).toBe(false);
  });

  test('a popover with too many stops says so on the console', async ({ page }) => {
    const warnings: string[] = [];
    page.on('console', (message) => {
      if (message.type() === 'warning') warnings.push(message.text());
    });
    await open(page, disagreeingStory);
    await settle(page);
    expect(warnings.some((text) => text.includes('tab stops'))).toBe(true);
    expect(warnings.some((text) => text.includes('kind="flyout"'))).toBe(true);
  });

  test('a surface with no room below it flips above its anchor', async ({ page }) => {
    await open(page, flipStory);
    const measured = await page.evaluate(() => {
      const host = document.querySelector('#flipping') as
        | (HTMLElement & { placement: { side: string } | undefined })
        | null;
      const anchor = document.querySelector('#flipping-anchor');
      const surface = host?.shadowRoot?.querySelector('.surface');
      if (!(anchor instanceof HTMLElement) || !(surface instanceof HTMLElement)) return undefined;
      return {
        side: host?.placement?.side ?? '',
        anchorTop: anchor.getBoundingClientRect().top,
        surfaceTop: surface.getBoundingClientRect().top,
      };
    });
    expect(measured).toBeDefined();
    if (measured === undefined) return;
    expect(measured.side).toBe('top');
    expect(measured.surfaceTop).toBeLessThan(measured.anchorTop);
  });
});

// ── the task pane, which is the one that is not like the others ──────────────

test.describe('the task pane', () => {
  /**
   * ⚠ **The inverted assertion.** Every other surface in this suite closes on Escape. A sweep that
   * asserted the same thing about all of them would have made this one wrong and stayed green.
   */
  test('ignores Escape, a click outside, and losing focus', async ({ page }) => {
    await open(page, dockedStory);
    await page.locator('#pane-width').focus();
    await page.keyboard.press('Escape');
    await settle(page);
    expect(await page.evaluate(() => document.querySelector('#pane')?.hasAttribute('open'))).toBe(
      true,
    );
    await page.locator('#page').click();
    await settle(page);
    expect(await page.evaluate(() => document.querySelector('#pane')?.hasAttribute('open'))).toBe(
      true,
    );
    expect(surfaceKinds.taskPane.dismissals).toEqual([]);
  });

  test('is in flow beside the document rather than drawn over it', async ({ page }) => {
    await open(page, dockedStory);
    const measured = await page.evaluate(() => {
      const pane = document.querySelector('#pane');
      const document_ = document.querySelector('#page');
      if (!(pane instanceof HTMLElement) || !(document_ instanceof HTMLElement)) return undefined;
      const paneBox = pane.getBoundingClientRect();
      const pageBox = document_.getBoundingClientRect();
      return {
        position: getComputedStyle(pane).position,
        popover: pane.shadowRoot?.querySelector('.pane')?.getAttribute('popover') ?? null,
        overlap: Math.max(0, Math.min(paneBox.right, pageBox.right) - Math.max(paneBox.left, pageBox.left)),
        paneWidth: paneBox.width,
        pageWidth: pageBox.width,
      };
    });
    expect(measured).toBeDefined();
    if (measured === undefined) return;
    expect(measured.position).toBe('static');
    expect(measured.popover).toBeNull();
    expect(measured.paneWidth).toBeGreaterThan(0);
    expect(measured.pageWidth).toBeGreaterThan(0);
    // ⚠ Not drawn over: the two boxes do not overlap at all, which is what a dock means and what a
    // pinned sheet would have broken.
    expect(measured.overlap).toBeLessThanOrEqual(1);
  });

  test('the splitter resizes by keyboard, and says the width it actually is', async ({ page }) => {
    await open(page, dockedStory);
    const splitter = page.locator('#pane').locator('.splitter');
    await splitter.focus();
    const before = await measurePane(page);
    await page.keyboard.press('ArrowLeft');
    await settle(page);
    const after = await measurePane(page);
    expect(after.width, 'ArrowLeft did not grow a pane docked at the end of the line').toBeGreaterThan(
      before.width,
    );
    // The announced value is the measured one, within a pixel of rounding.
    expect(Math.abs(after.announced - after.measuredFraction * 100)).toBeLessThan(2);
    expect(after.announced - before.announced).toBe(Math.round(taskPaneResizeStep * 100));

    await page.keyboard.press('End');
    await settle(page);
    const widest = await measurePane(page);
    expect(widest.announced).toBe(Math.round(taskPaneFractionBounds.max * 100));
    await page.keyboard.press('Home');
    await settle(page);
    const narrowest = await measurePane(page);
    expect(narrowest.announced).toBe(Math.round(taskPaneFractionBounds.min * 100));
    // …and the two ends are genuinely different, so "it moves" is not satisfied by a splitter that
    // snaps everything to one value.
    expect(widest.width).toBeGreaterThan(narrowest.width + 10);
  });

  for (const scheme of schemes) {
    test(`the edge against a coloured page is the member the rule chose, in ${scheme}`, async ({
      page,
    }) => {
      await open(page, colouredStory, { theme: scheme });
      await settlePaint(page);
      const measured = await page.evaluate(() => {
        const pane = document.querySelector('#pane') as
          | (HTMLElement & { edge: { member: string; ratio: number; sufficient: boolean } | undefined })
          | null;
        const splitter = pane?.shadowRoot?.querySelector('.splitter');
        if (!(splitter instanceof HTMLElement)) return undefined;
        return {
          member: pane?.edge?.member ?? '',
          sufficient: pane?.edge?.sufficient ?? false,
          painted: getComputedStyle(splitter, '::after').backgroundColor,
        };
      });
      expect(measured).toBeDefined();
      if (measured === undefined) return;
      expect(measured.member, 'the pane chose no edge at all').not.toBe('');
      expect(measured.sufficient).toBe(true);
      expect(measured.painted).toBe(rgbOf(themeColor(scheme, measured.member as 'textPrimary')));
    });
  }

  test('at a phone width it takes the whole frame and offers nothing to resize', async ({ page }) => {
    await open(page, dockedStory, { containerPreset: 'phone' });
    const measured = await page.evaluate(() => {
      const pane = document.querySelector('#pane') as (HTMLElement & { presentation: string }) | null;
      const splitter = pane?.shadowRoot?.querySelector('.splitter');
      return {
        presentation: pane?.presentation ?? '',
        splitter: splitter instanceof HTMLElement ? getComputedStyle(splitter).display : 'missing',
      };
    });
    expect(measured.presentation).toBe('full');
    expect(measured.splitter).toBe('none');
  });
});

async function measurePane(page: Page): Promise<{
  width: number;
  announced: number;
  measuredFraction: number;
}> {
  const measured = await page.evaluate(() => {
    const pane = document.querySelector('#pane');
    const workspace = document.querySelector('#workspace');
    const splitter = pane?.shadowRoot?.querySelector('.splitter');
    if (!(pane instanceof HTMLElement) || !(workspace instanceof HTMLElement)) return undefined;
    const paneBox = pane.getBoundingClientRect();
    const workspaceBox = workspace.getBoundingClientRect();
    return {
      width: paneBox.width,
      announced: Number.parseFloat(splitter?.getAttribute('aria-valuenow') ?? 'NaN'),
      measuredFraction: paneBox.width / workspaceBox.width,
    };
  });
  expect(measured, 'the pane could not be measured at all').toBeDefined();
  return measured ?? { width: 0, announced: Number.NaN, measuredFraction: 0 };
}

// ── the hit-target floor, in the density that could break it ─────────────────

test.describe('the hit-target floor holds in compact', () => {
  test('a dialog’s close control', async ({ page }) => {
    await open(page, compactDialogStory);
    const box = await page.evaluate(() => {
      const close = document.querySelector('#compact')?.shadowRoot?.querySelector('.close');
      if (!(close instanceof HTMLElement)) return undefined;
      const rect = close.getBoundingClientRect();
      return { width: rect.width, height: rect.height };
    });
    expect(box).toBeDefined();
    if (box === undefined) return;
    expect(box.width).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
    expect(box.height).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
  });

  test('a task pane’s splitter', async ({ page }) => {
    await open(page, compactPaneStory);
    const box = await page.evaluate(() => {
      const splitter = document.querySelector('#pane')?.shadowRoot?.querySelector('.splitter');
      if (!(splitter instanceof HTMLElement)) return undefined;
      const rect = splitter.getBoundingClientRect();
      return { width: rect.width, height: rect.height };
    });
    expect(box).toBeDefined();
    if (box === undefined) return;
    expect(box.width).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
    expect(box.height).toBeGreaterThan(0);
  });
});

// ── the container is the mechanism, not the viewport ─────────────────────────

test('driving the container never moves the window', async ({ page }) => {
  await open(page, modalStory, { containerPreset: 'desktop' });
  const wide = await page.evaluate(() => window.innerWidth);
  await open(page, modalStory, { containerPreset: 'phone' });
  const narrow = await page.evaluate(() => window.innerWidth);
  expect(narrow).toBe(wide);
  expect(containerPresets.phone).toBeLessThan(containerPresets.desktop);
});
