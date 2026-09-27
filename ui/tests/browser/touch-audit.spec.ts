import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { expect, test, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import {
  catalogueComponents,
  describeTarget,
  interactiveSelector,
  meetsTargetSize,
  type AuditKind,
  type TargetBox,
} from '../../src/mobile/touch-audit.ts';
import { accessibleHitTargetMinimum } from '../../src/foundations/density.ts';
import { containerPresets } from '../../src/harness/presets.ts';

/**
 * **The touch-target audit** — a sweep over *every* component in the catalogue at phone width, in
 * both themes.
 *
 * MJXOFF-194 calls it the most valuable half of the child and states the trap it is written
 * against: *a sweep over the components this child happens to touch proves nothing*. So the sweep
 * runs over the **built story index** — the catalogue's own list of itself, the same source the
 * a11y sweep uses — measures every interactive target it finds, attributes each to the component
 * that owns it, and then **accounts for the whole registry at the end.**
 *
 * ## Four ways it fails, and none of them is *found nothing*
 *
 * | Failure | Where |
 * |---|---|
 * | a component `src/` defines that the registry does not know | `tests/mobile.test.ts`, before a browser starts |
 * | a registered component no story renders | `every component was accounted for`, below |
 * | a component declared target-free that grew a target | the same test |
 * | a target under the floor | every per-story test |
 *
 * Plus two anti-vacuity assertions, because the programme's recurring defect is a helper whose
 * failure mode is finding nothing: the sweep asserts it saw a **large** number of targets overall,
 * and asserts that it saw at least one for **every** component that claims to have them.
 *
 * ## Why the geometry is evaluated in the page and the rule is not
 *
 * `meetsTargetSize` is a pure function in `src/mobile/touch-audit.ts`, unit-tested against
 * hand-worked cases including the exact boundary. The page only ever hands back boxes. A rule
 * implemented inside `page.evaluate` would be a rule no unit test could reach, and the WCAG spacing
 * exception is exactly the kind of arithmetic that is easy to get subtly wrong.
 */

const stories = builtStories();

interface CollectedTarget {
  readonly tag: string | null;
  readonly kind: AuditKind | null;
  readonly where: string;
  readonly box: TargetBox;
}

/**
 * ⚠ **The aggregate is on disk, and that is not paranoia.**
 *
 * The first version of this sweep accumulated into module-level `Set`s, which is the obvious thing
 * to write and is **wrong here**: Playwright tears a worker process down after a failing test and
 * starts a fresh one, so every module-level total resets at each failure. The accounting test then
 * saw only whatever had accumulated since the last one and reported forty components as never
 * rendered — a spectacular false positive that would have been a spectacular *false negative* on a
 * green run, because a run with no failures never restarts a worker and the aggregate looks
 * complete.
 *
 * A file survives the restart. The directory is cleared by the first test in the file, and the last
 * one requires to find exactly as many records as there are stories — so a partial run fails rather
 * than reporting a partial answer as a whole one.
 */
const ledger = resolve(import.meta.dirname, '../../.touch-audit');

interface StoryRecord {
  readonly story: string;
  readonly present: string[];
  readonly withTargets: string[];
  readonly targetCount: number;
}

function record(entry: StoryRecord): void {
  mkdirSync(ledger, { recursive: true });
  const name = createHash('sha1').update(entry.story).digest('hex').slice(0, 16);
  writeFileSync(resolve(ledger, `${name}.json`), JSON.stringify(entry), 'utf8');
}

function ledgerEntries(): StoryRecord[] {
  if (!existsSync(ledger)) return [];
  return readdirSync(ledger)
    .filter((name) => name.endsWith('.json'))
    .map((name) => JSON.parse(readFileSync(resolve(ledger, name), 'utf8')) as StoryRecord);
}

const registry = catalogueComponents.map((component) => ({
  tag: component.tag,
  kind: component.kind,
}));

async function collect(page: Page): Promise<{
  targets: CollectedTarget[];
  present: string[];
}> {
  return page.evaluate(
    ({ selector, table }) => {
      const kinds = new Map(table.map((entry) => [entry.tag, entry.kind]));

      /** Every element in the tree, light DOM and every shadow root, depth first. */
      function everything(root: ParentNode, into: Element[] = []): Element[] {
        for (const element of Array.from(root.querySelectorAll('*'))) {
          into.push(element);
          const shadow = (element as HTMLElement).shadowRoot;
          if (shadow !== null) everything(shadow, into);
        }
        return into;
      }

      /**
       * Which registered component owns this element.
       *
       * Two rules, and **both** were found by watching this fail:
       *
       * 1. **A registered element owns itself.** `<mjx-menu-item>`, `<mjx-splitter>`,
       *    `<mjx-scrollbar>` and `<mjx-status-segment>` put their role and their tab stop on their
       *    own **host**, not on something inside their shadow root — so a walk that only looked
       *    upward attributed them to nobody and the accounting reported seven components as never
       *    showing a target.
       * 2. **A shadow root confers ownership; light-DOM ancestry does not.** `<mjx-screentip>` wraps
       *    the control it describes, so an ancestor walk reported the screentip as having grown four
       *    buttons that belonged to the story. A component owns what it *built* — what is inside its
       *    own shadow root — plus itself.
       */
      function owner(element: Element): string | null {
        if (kinds.has(element.localName)) return element.localName;
        let node: Node | null = element;
        while (node !== null) {
          const parent: Node | null = node.parentNode;
          if (parent === null && node instanceof ShadowRoot) {
            const host: Element = node.host;
            if (kinds.has(host.localName)) return host.localName;
            node = host;
            continue;
          }
          node = parent;
        }
        return null;
      }

      function describe(element: Element): string {
        const own = owner(element);
        const classes = element.className === '' ? '' : `.${String(element.className).split(' ').join('.')}`;
        return `${own ?? '(unowned)'} › ${element.localName}${classes}`;
      }

      const root = document.querySelector('#storybook-root');
      if (root === null) return { targets: [], present: [] };

      const all = everything(root);
      const present = [...new Set(all.map((element) => element.localName))].filter((tag) =>
        kinds.has(tag),
      );

      const targets: {
        tag: string | null;
        kind: string | null;
        where: string;
        box: { x: number; y: number; width: number; height: number };
      }[] = [];

      for (const element of all) {
        if (!element.matches(selector)) continue;
        const style = getComputedStyle(element);
        if (style.visibility === 'hidden' || style.display === 'none') continue;
        const rect = element.getBoundingClientRect();
        // A zero-sized box is an element that is not on screen — a closed popup's content, a
        // virtualised row that was recycled. Measuring it would report a 0 x 0 failure about
        // something nobody can press.
        if (rect.width <= 0 || rect.height <= 0) continue;
        const tag = owner(element);
        targets.push({
          tag,
          kind: tag === null ? null : (kinds.get(tag) ?? null),
          where: describe(element),
          box: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
        });
      }

      return { targets, present };
    },
    { selector: interactiveSelector, table: registry },
  ) as Promise<{ targets: CollectedTarget[]; present: string[] }>;
}

function measure(targets: readonly CollectedTarget[], theme: string, story: string): string[] {
  // Only the components that claim to have targets are ASSERTED on: a harness control is not
  // shipped chrome, and a target belonging to nothing registered is a story's own scaffolding.
  //
  // ⚠ But **every** target on the page is a neighbour, whoever owns it. WCAG's spacing exception is
  // about what a thumb might hit by mistake, and a thumb does not know which component a button
  // came from — so restricting the neighbour set to audited targets would quietly widen the
  // exception every time an audited control sat next to one that is not.
  const failures: string[] = [];
  for (const [index, target] of targets.entries()) {
    if (target.kind !== 'targets') continue;
    const neighbours = targets
      .filter((_other, position) => position !== index)
      .map((other) => other.box);
    const verdict = meetsTargetSize(target.box, neighbours);
    if (!verdict.ok) {
      failures.push(`  [${theme}] ${story} — ${describeTarget(target.where, target.box, verdict)}`);
    }
  }
  return failures;
}

test.describe('the touch-target sweep', () => {
  test('the sweep has a catalogue to sweep, and a registry to check it against', () => {
    // Declared first, so it clears last run's ledger exactly once before anything is written to it.
    rmSync(ledger, { recursive: true, force: true });
    expect(stories.length).toBeGreaterThan(0);
    expect(catalogueComponents.length).toBeGreaterThan(40);
    expect(registry.filter((entry) => entry.kind === 'targets').length).toBeGreaterThan(20);
  });

  for (const story of stories) {
    test(`${story.title} · ${story.name}`, async ({ page }) => {
      const where = `${story.title} · ${story.name}`;
      const failures: string[] = [];
      const present = new Set<string>();
      const withTargets = new Set<string>();
      let targetCount = 0;

      const take = (collected: { targets: CollectedTarget[]; present: string[] }, theme: string): void => {
        for (const tag of collected.present) present.add(tag);
        targetCount += collected.targets.length;
        for (const target of collected.targets) {
          if (target.tag === null) continue;
          if (target.kind === 'targets') withTargets.add(target.tag);
          if (target.kind === 'presentational' || target.kind === 'descriptor') {
            failures.push(
              `  [${theme}] ${target.tag} is declared ${target.kind} in ` +
                `src/mobile/touch-audit.ts and contributed an interactive target: ` +
                `${target.where}. Either the component grew a control, or its audit kind is wrong.`,
            );
          }
        }
        failures.push(...measure(collected.targets, theme, where));
      };

      await openStory(page, story.id, { containerPreset: 'phone', theme: 'light' });
      const light = await collect(page);
      take(light, 'light');

      // The dark pass is skipped only where the light one found nothing to measure, which is the
      // one case in which it could not possibly find anything either.
      if (light.targets.some((target) => target.kind === 'targets')) {
        await openStory(page, story.id, { containerPreset: 'phone', theme: 'dark' });
        take(await collect(page), 'dark');
      }

      // Recorded BEFORE the assertion, so a story that fails its measurement still contributes what
      // it saw — otherwise one bad target would make the accounting test report the component as
      // never rendered, which is a second, misleading failure about the same defect.
      record({ story: where, present: [...present], withTargets: [...withTargets], targetCount });

      expect(
        failures,
        `Touch-target findings at ${String(containerPresets.phone)} px, against the ` +
          `${String(accessibleHitTargetMinimum)} px floor on BOTH axes with WCAG 2.2's spacing ` +
          `exception:\n${failures.join('\n')}`,
      ).toEqual([]);
    });
  }

  /**
   * ⚠ **Declared last, and reading from disk rather than from memory.** See the note on `ledger`:
   * Playwright restarts a worker after a failure, so an in-memory aggregate is complete only on a
   * run with nothing wrong in it — which is the worst possible property for the assertion that says
   * the sweep covered everything.
   */
  test('every component was accounted for', () => {
    const entries = ledgerEntries();
    expect(
      entries.length,
      'The ledger holds fewer records than there are stories, so this is a partial run and the ' +
        'accounting below would be about a sample rather than the catalogue.',
    ).toBe(stories.length);

    const totalTargets = entries.reduce((sum, entry) => sum + entry.targetCount, 0);
    // Anti-vacuity. A collector that had silently stopped matching would report zero here and every
    // per-story test above would have passed by measuring nothing.
    expect(
      totalTargets,
      'The sweep found almost no interactive targets across the whole catalogue, which cannot be ' +
        'right. The collector, not the components, is what to look at.',
    ).toBeGreaterThan(200);

    const present = new Set(entries.flatMap((entry) => entry.present));
    const withTargets = new Set(entries.flatMap((entry) => entry.withTargets));

    const neverSeen = catalogueComponents
      .filter((component) => !present.has(component.tag))
      .map((component) => component.tag);
    expect(
      neverSeen,
      'These components are registered in src/ and no story in the catalogue renders one, so the ' +
        'touch audit has never looked at them. Every component in this catalogue is supposed to ' +
        'be auditable.\n  ' + neverSeen.join('\n  '),
    ).toEqual([]);

    const claimedButSilent = catalogueComponents
      .filter((component) => component.kind === 'targets' && !withTargets.has(component.tag))
      .map((component) => component.tag);
    expect(
      claimedButSilent,
      'These components declare that they have interactive targets and contributed none in any ' +
        'story. Either no story shows them in a state where their controls exist — in which case ' +
        'the audit is not looking at them and a story is needed — or the declaration is wrong.\n  ' +
        claimedButSilent.join('\n  '),
    ).toEqual([]);
  });
});
