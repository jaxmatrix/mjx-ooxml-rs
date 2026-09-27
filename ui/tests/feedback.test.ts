import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

import {
  ScreentipWarmth,
  ToastQueue,
  emptyStateRole,
  feedbackTimingCss,
  feedbackTimingMilliseconds,
  feedbackTimingMultiples,
  feedbackTimingNames,
  feedbackTimingProperties,
  feedbackTimingValue,
  feedbackTypeRoles,
  emptyStateCss,
  formatProgressPercent,
  indeterminateSpanFraction,
  miniToolbarCss,
  miniToolbarClearanceOrder,
  nonTextMinimum,
  politenessRoles,
  progressCss,
  progressFraction,
  progressValueText,
  screentipClearanceOrder,
  screentipCss,
  screentipDelayFor,
  toastCss,
  toastEdgeContrast,
  toastSignals,
  toastStackCeiling,
  toastToneNames,
  toastTones,
  toneColourSeparation,
  type ToastTone,
} from '../src/feedback/feedback-model.ts';
import {
  overlapArea,
  placeClearOfAnchor,
  placeFloating,
  type LogicalSide,
  type Rect,
} from '../src/overlay/floating.ts';
import { durationMultiple, resolveDurationMilliseconds } from '../src/foundations/motion.ts';
import { typeRoleClass } from '../src/foundations/typography.ts';
import { tokens, type ColorScheme } from '../tokens/tokens.ts';

/**
 * The feedback family, proved in Node.
 *
 * Four kinds of assertion live here, and each is one a browser cannot make better:
 *
 * * **the clearance arithmetic, swept.** *"The toolbar never covers the selection"* is a property of
 *   two rectangles and a boundary, and a browser can drive one arrangement at a time. This drives
 *   four hundred and, beside the ceiling, asserts that the sweep contains cases of **both** answers
 *   — because *"every clear placement has zero overlap"* is satisfied by a sweep in which nothing
 *   was ever clear;
 * * **time, without waiting.** The screentip's schedule and the toast queue take `now` as an
 *   argument, so their whole behaviour is a sequence of instants rather than a test that sleeps and
 *   samples the end;
 * * **the tone table's own consistency**, including the measurement that says *why* a tone may not
 *   be carried by colour: the two accents this palette offers are within a hair of each other in
 *   luminance, so a contrast gate cannot tell them apart and neither can some readers;
 * * **the arithmetic a single value would not exercise.** `progressFraction` has six branches and
 *   any one value visits at most one of them.
 */

const schemes: readonly ColorScheme[] = ['light', 'dark'];

// ── the clearance arithmetic ─────────────────────────────────────────────────

const boundary: Rect = { x: 0, y: 0, width: 1000, height: 600 };
const toolbar = { width: 200, height: 40 };

function rectOfPlacement(x: number, y: number, size: { width: number; height: number }): Rect {
  return { x, y, width: size.width, height: size.height };
}

function clearanceFor(anchor: Rect, order: readonly LogicalSide[] = miniToolbarClearanceOrder) {
  return placeClearOfAnchor(
    { anchor, floating: toolbar, boundary, align: 'center', direction: 'ltr', gap: 8 },
    order,
  );
}

describe('a box that must not cover its anchor', () => {
  it('takes the first side in the order when that side is clear', () => {
    const selection: Rect = { x: 300, y: 300, width: 240, height: 20 };
    const result = clearanceFor(selection);
    expect(result.clear).toBe(true);
    expect(result.overlap).toBe(0);
    expect(result.requested).toBe(miniToolbarClearanceOrder[0]);
    expect(result.tried).toBe(1);
    // Above, which is where a mini toolbar belongs.
    expect(result.placement.side).toBe('top');
  });

  it('flips inside the first candidate rather than moving on to the second', () => {
    // No room above at all: `placeFloating` itself takes the other side, so the first candidate is
    // still the one that answers. A clearance search that moved on would report `tried: 2` and
    // would be doing the flip twice.
    const selection: Rect = { x: 300, y: 2, width: 240, height: 20 };
    const result = clearanceFor(selection);
    expect(result.clear).toBe(true);
    expect(result.tried).toBe(1);
    expect(result.placement.flipped).toBe(true);
    expect(result.placement.side).toBe('bottom');
  });

  /**
   * ⚠ **The assertion that says this function earns its existence.**
   *
   * A selection as tall as its boundary has no room on either block side, so `placeFloating`'s
   * final clamp puts the box back *on top of* the selection — the exact defect. The clearance
   * search finds an inline side instead. Asserting the second without asserting the first would be
   * asserting that a placement is clear without ever showing that the primitive's own answer is
   * not.
   */
  it('finds an inline side where the primitive alone would land on the anchor', () => {
    const selection: Rect = { x: 400, y: 0, width: 120, height: 600 };
    const naive = placeFloating({
      anchor: selection,
      floating: toolbar,
      boundary,
      side: 'blockStart',
      align: 'center',
      direction: 'ltr',
      gap: 8,
    });
    expect(
      overlapArea(rectOfPlacement(naive.x, naive.y, toolbar), selection),
      'placeFloating alone was already clear here, so this case proves nothing',
    ).toBeGreaterThan(0);

    const result = clearanceFor(selection);
    expect(result.clear).toBe(true);
    expect(result.overlap).toBe(0);
    expect(result.requested).toBe('inlineEnd');
    expect(result.tried).toBe(3);
  });

  it('reports failure rather than hiding it when the anchor fills the boundary', () => {
    const result = clearanceFor(boundary);
    expect(result.clear).toBe(false);
    expect(result.overlap).toBeGreaterThan(0);
    expect(result.tried).toBe(miniToolbarClearanceOrder.length);
  });

  it('an empty order is told apart from a genuine four-sided failure', () => {
    const result = clearanceFor(boundary, []);
    expect(result.clear).toBe(false);
    expect(result.tried).toBe(0);
    expect(result.overlap).toBe(Number.POSITIVE_INFINITY);
  });

  /**
   * The sweep, with its anti-vacuity clause beside it.
   *
   * U06's lesson in this child's costume: *"a ceiling satisfied by zero previews."* A sweep that
   * asserts *"every clear placement has zero overlap"* is satisfied by a sweep in which nothing was
   * ever clear, and a sweep that asserts *"the covering flag is only set when covering"* is
   * satisfied by one that never set it. So both counts are asserted to be positive.
   */
  it('every clear placement really is clear, and both answers occur', () => {
    let clear = 0;
    let covered = 0;
    for (let x = 0; x <= 900; x += 100) {
      for (let y = 0; y <= 500; y += 100) {
        for (const width of [40, 240, 900]) {
          for (const height of [20, 200, 600]) {
            const selection: Rect = { x, y, width, height };
            const result = clearanceFor(selection);
            const box = rectOfPlacement(result.placement.x, result.placement.y, toolbar);
            const overlap = overlapArea(box, selection);
            expect(result.overlap).toBeCloseTo(overlap, 6);
            if (result.clear) {
              clear += 1;
              expect(overlap, `${JSON.stringify(selection)} claimed clear`).toBe(0);
            } else {
              covered += 1;
              expect(overlap, `${JSON.stringify(selection)} claimed covering`).toBeGreaterThan(0);
              // …and every side really was tried before giving up.
              expect(result.tried).toBe(miniToolbarClearanceOrder.length);
            }
          }
        }
      }
    }
    expect(clear, 'the sweep found no clear placement, so its clear branch proves nothing').toBeGreaterThan(0);
    expect(covered, 'the sweep found no covering placement, so its failure branch proves nothing').toBeGreaterThan(0);
  });

  it('the two orders are the four sides, and they differ in which one they want first', () => {
    expect([...miniToolbarClearanceOrder].sort()).toEqual([...screentipClearanceOrder].sort());
    expect(miniToolbarClearanceOrder[0]).toBe('blockStart');
    expect(screentipClearanceOrder[0]).toBe('blockEnd');
  });
});

// ── the screentip's delay ────────────────────────────────────────────────────

const schedule = { appear: 600, warm: 900 };

describe('a screentip waits, and the warm-up is why a toolbar is readable', () => {
  it('waits the full delay when nothing has been shown', () => {
    expect(screentipDelayFor(1000, undefined, schedule)).toBe(schedule.appear);
  });

  /**
   * Sampled in the middle, at both boundaries, and past them — the identity-value discipline
   * applied to time. A test at `warm / 2` alone would pass for a rule that never expires, and a
   * test at `warm * 10` alone would pass for one that never warms.
   */
  it.each([
    [0, 0],
    [schedule.warm / 2, 0],
    [schedule.warm - 1, 0],
    [schedule.warm, 0],
    [schedule.warm + 1, schedule.appear],
    [schedule.warm * 4, schedule.appear],
  ])('at %i ms since the last tip, waits %i ms', (since, expected) => {
    expect(screentipDelayFor(10_000 + since, 10_000, schedule)).toBe(expected);
  });

  it('a clock that went backwards waits rather than treating a negative age as warm', () => {
    expect(screentipDelayFor(9_000, 10_000, schedule)).toBe(schedule.appear);
  });

  it('a sequence: the first waits, the second does not, the third waits again', () => {
    const warmth = new ScreentipWarmth();
    expect(warmth.delayFor(0, schedule)).toBe(schedule.appear);
    warmth.noteHidden(1_000);
    expect(warmth.delayFor(1_400, schedule)).toBe(0);
    expect(warmth.delayFor(1_000 + schedule.warm + 1, schedule)).toBe(schedule.appear);
    warmth.reset();
    expect(warmth.lastHiddenAt).toBeUndefined();
    expect(warmth.delayFor(1_400, schedule)).toBe(schedule.appear);
  });
});

// ── the toast queue ──────────────────────────────────────────────────────────

const shortDwell = 1_000;
const longDwell = 1_500;

function dwellFor(tone: ToastTone): number | undefined {
  const declared = toastTones[tone].dwell;
  if (declared === 'persistent') return undefined;
  return declared === 'toastDwellLong' ? longDwell : shortDwell;
}

describe('the toast queue', () => {
  it('keeps insertion order, which is also the order it renders in', () => {
    const queue = new ToastQueue();
    queue.push({ tone: 'info', message: 'one' }, 0, dwellFor);
    queue.push({ tone: 'info', message: 'two' }, 10, dwellFor);
    expect(queue.entries.map((entry) => entry.message)).toEqual(['one', 'two']);
  });

  /**
   * ⚠ **The assertion MJXOFF-189 asks for by name.** Every entry carries its own deadline, so a
   * second toast does not restart the first one's clock — which is what an implementation with a
   * single shared timeout does, and what a test that only waited for the end would never see.
   */
  it('a second toast does not cancel the first one’s timer', () => {
    const queue = new ToastQueue();
    queue.push({ tone: 'info', message: 'first' }, 0, dwellFor);
    queue.push({ tone: 'info', message: 'second' }, shortDwell / 2, dwellFor);

    // Halfway through the first one's dwell: both are still here.
    expect(queue.expire(shortDwell / 2)).toEqual([]);
    expect(queue.size).toBe(2);

    // The first one's deadline exactly. The first goes; the second, pushed later, does not.
    const gone = queue.expire(shortDwell);
    expect(gone.map((departure) => departure.entry.message)).toEqual(['first']);
    expect(queue.entries.map((entry) => entry.message)).toEqual(['second']);

    // …and the second leaves at its own deadline rather than at the first one's.
    expect(queue.expire(shortDwell + shortDwell / 2 - 1)).toEqual([]);
    expect(queue.expire(shortDwell + shortDwell / 2).map((d) => d.entry.message)).toEqual(['second']);
    expect(queue.size).toBe(0);
  });

  it('a warning stays longer than an information message, from the tone table', () => {
    const queue = new ToastQueue();
    queue.push({ tone: 'info', message: 'quick' }, 0, dwellFor);
    queue.push({ tone: 'warning', message: 'slow' }, 0, dwellFor);
    expect(queue.expire(shortDwell).map((d) => d.entry.message)).toEqual(['quick']);
    expect(queue.entries.map((entry) => entry.message)).toEqual(['slow']);
    expect(queue.expire(longDwell).map((d) => d.entry.message)).toEqual(['slow']);
  });

  it('an error never leaves on its own, however long anyone waits', () => {
    const queue = new ToastQueue();
    const { entry } = queue.push({ tone: 'error', message: 'it failed' }, 0, dwellFor);
    expect(entry.expiresAt).toBeUndefined();
    expect(queue.expire(Number.MAX_SAFE_INTEGER)).toEqual([]);
    expect(queue.size).toBe(1);
    expect(queue.dismiss(entry.id)?.message).toBe('it failed');
    expect(queue.size).toBe(0);
    expect(queue.dismiss(entry.id)).toBeUndefined();
  });

  it('holds a ceiling, and the ceiling is a positive number of real entries', () => {
    const queue = new ToastQueue();
    for (let index = 0; index < toastStackCeiling + 2; index += 1) {
      queue.push({ tone: 'info', message: `message ${String(index)}` }, index, dwellFor);
    }
    expect(toastStackCeiling).toBeGreaterThan(0);
    expect(queue.size).toBe(toastStackCeiling);
    // The oldest went, and the newest is here — a ceiling that kept the wrong end would have the
    // same size and be useless.
    expect(queue.entries[0]?.message).toBe('message 2');
    expect(queue.entries.at(-1)?.message).toBe(`message ${String(toastStackCeiling + 1)}`);
  });

  /**
   * The half of the ceiling that is a design decision rather than arithmetic.
   *
   * An error a person has not read must not be shunted off the screen by three confirmations, so
   * the ceiling retires the oldest entry that **leaves on its own** before it touches a persistent
   * one. A ceiling that simply took the head of the queue would pass the size assertion above and
   * lose the only message that mattered.
   */
  it('the ceiling retires a confirmation before it retires an error', () => {
    const queue = new ToastQueue();
    queue.push({ tone: 'error', message: 'it failed' }, 0, dwellFor);
    for (let index = 0; index < toastStackCeiling; index += 1) {
      queue.push({ tone: 'success', message: `saved ${String(index)}` }, index + 1, dwellFor);
    }
    expect(queue.size).toBe(toastStackCeiling);
    expect(queue.entries.map((entry) => entry.tone)).toContain('error');
    expect(queue.entries[0]?.message).toBe('it failed');
    expect(queue.entries.map((entry) => entry.message)).not.toContain('saved 0');
  });

  it('reports the next deadline, and reports none when nothing has one', () => {
    const queue = new ToastQueue();
    expect(queue.nextExpiry()).toBeUndefined();
    queue.push({ tone: 'error', message: 'stays' }, 0, dwellFor);
    expect(queue.nextExpiry()).toBeUndefined();
    queue.push({ tone: 'warning', message: 'later' }, 0, dwellFor);
    queue.push({ tone: 'info', message: 'sooner' }, 0, dwellFor);
    expect(queue.nextExpiry()).toBe(shortDwell);
    queue.clear();
    expect(queue.size).toBe(0);
    expect(queue.nextExpiry()).toBeUndefined();
  });
});

// ── the tones ────────────────────────────────────────────────────────────────

describe('the four tones', () => {
  it('every tone’s edge clears the non-text minimum against the card it is drawn on', () => {
    for (const scheme of schemes) {
      for (const tone of toastToneNames) {
        const ratio = toastEdgeContrast(tone, scheme);
        expect(ratio, `${tone} in ${scheme} is ${String(ratio)} : 1`).toBeGreaterThanOrEqual(
          nonTextMinimum,
        );
      }
    }
  });

  /**
   * ⚠ **The measurement that makes the non-colour signals necessary rather than ceremonial.**
   *
   * A contrast ratio sees luminance and nothing else, so two colours that differ only in hue are
   * *identical* to it — and to a reader with a colour deficiency they may be close to identical
   * too. This palette's two accents are exactly that pair. Asserting the separation is **below**
   * the floor is the honest way round: it records the fact the design is built on, and it will fail
   * loudly if a re-seed ever makes colour a usable signal, at which point this reasoning should be
   * revisited rather than silently kept.
   */
  it('colour alone could not tell a warning from a success, and the number says so', () => {
    for (const scheme of schemes) {
      const separation = toneColourSeparation('warning', 'success', scheme);
      expect(separation, `${scheme} separation is ${String(separation)}`).toBeLessThan(
        nonTextMinimum,
      );
    }
  });

  it('so every pair of tones differs in something that is not a colour', () => {
    for (const first of toastToneNames) {
      for (const second of toastToneNames) {
        if (first === second) continue;
        const a = toastSignals(first);
        const b = toastSignals(second);
        const differences = a.filter((signal) => !b.includes(signal));
        expect(
          differences.length,
          `${first} and ${second} share every non-colour signal: ${a.join(', ')}`,
        ).toBeGreaterThan(0);
      }
    }
  });

  it('exactly one tone is assertive, and it is the one that never leaves', () => {
    const assertive = toastToneNames.filter((tone) => toastTones[tone].politeness === 'assertive');
    expect(assertive).toEqual(['error']);
    for (const tone of toastToneNames) {
      const spec = toastTones[tone];
      expect(spec.dwell === 'persistent').toBe(spec.politeness === 'assertive');
    }
  });

  it('the two politenesses have the two roles, and neither is the other’s', () => {
    expect(politenessRoles.polite).toBe('status');
    expect(politenessRoles.assertive).toBe('alert');
    expect(new Set(Object.values(politenessRoles)).size).toBe(2);
  });

  it('every tone names a member the generated palette has, in both schemes', () => {
    for (const scheme of schemes) {
      for (const tone of toastToneNames) {
        expect(tokens.theme[scheme]).toHaveProperty(toastTones[tone].edge);
      }
    }
  });
});

// ── progress ─────────────────────────────────────────────────────────────────

describe('the progress arithmetic', () => {
  /**
   * ⚠ **Six branches, and a bar tested at one value visits at most one of them.** That is the
   * identity-value trap MJXOFF-189 names, written out as a table.
   */
  it.each([
    ['nothing done', 0, 1, 0],
    ['a quarter', 0.25, 1, 0.25],
    ['halfway', 0.5, 1, 0.5],
    ['all of it', 1, 1, 1],
    ['past the end', 4, 1, 1],
    ['below the start', -3, 1, 0],
    ['three of seven', 3, 7, 3 / 7],
    ['a hundred and twenty-eight of five hundred and twelve', 128, 512, 0.25],
    ['a maximum of zero is a task with nothing in it', 1, 0, 0],
    ['a negative maximum', 1, -5, 0],
    ['a value that is not a number', Number.NaN, 1, 0],
    ['an infinite maximum', 1, Number.POSITIVE_INFINITY, 0],
  ])('%s', (_name, value, max, expected) => {
    expect(progressFraction(value, max)).toBeCloseTo(expected, 10);
  });

  it('the fraction is monotone in the value, which is what makes a bar readable', () => {
    let previous = -1;
    for (let value = 0; value <= 10; value += 1) {
      const fraction = progressFraction(value, 10);
      expect(fraction).toBeGreaterThanOrEqual(previous);
      previous = fraction;
    }
    expect(previous).toBe(1);
  });

  it.each([
    [0, '0%'],
    [0.004, '0%'],
    [0.005, '1%'],
    [1 / 3, '33%'],
    [0.455, '46%'],
    [1, '100%'],
    [4, '100%'],
    [Number.NaN, '0%'],
  ])('formats %s as %s', (fraction, expected) => {
    expect(formatProgressPercent(fraction)).toBe(expected);
  });

  it('an indeterminate bar announces no quantity at all', () => {
    expect(progressValueText('determinate', 0.5, 'Uploading')).toBe('Uploading: 50%');
    const working = progressValueText('indeterminate', 0.5, 'Uploading');
    expect(working).toBe('Uploading: working');
    expect(working).not.toMatch(/\d/);
  });

  it('the indeterminate span is neither a full track nor a fraction anyone reads as one', () => {
    expect(indeterminateSpanFraction).toBeGreaterThan(0);
    expect(indeterminateSpanFraction).toBeLessThan(1);
    for (const readable of [0.25, 1 / 3, 0.5, 0.75]) {
      expect(Math.abs(indeterminateSpanFraction - readable)).toBeGreaterThan(0.02);
    }
  });

  /**
   * ⚠ **U09's lesson, in this child's stylesheet.** A `@media` block changes no specificity, so a
   * reduced-motion rule emitted *above* the rule it overrides simply loses at equal specificity —
   * and an indeterminate bar would keep sweeping for a person who asked their operating system to
   * stop moving things, with every other assertion green.
   *
   * The selector text is asserted to be **identical**, which is what makes the two rules equal in
   * specificity by construction rather than by anybody's arithmetic; the order is then the only
   * thing left to decide the winner, and it is asserted too.
   */
  it('the reduced-motion rule is last, and its selector is the one it must beat', () => {
    const selector = ".track[data-kind='indeterminate'] > .indicator";
    const base = progressCss.indexOf(selector);
    const media = progressCss.indexOf('@media (prefers-reduced-motion: reduce)');
    expect(base, 'the indeterminate rule is not in the stylesheet').toBeGreaterThan(-1);
    expect(media, 'the reduced-motion block is not in the stylesheet').toBeGreaterThan(base);
    expect(progressCss.indexOf(selector, media)).toBeGreaterThan(media);
    expect(progressCss.slice(media)).toContain('animation-name: none');
  });
});

/**
 * ⚠ **The rule a browser gate found missing, and the ordering it depends on.**
 *
 * `<mjx-empty-state>` hid its action, the attribute was set, the accessibility tree was right, and
 * the button was on screen — because the UA's `[hidden] { display: none }` is in the *user-agent*
 * origin and any author rule at all beats it, so `.action { display: inline-flex }` had switched
 * `hidden` off for every element wearing that class.
 *
 * The restatement scores the same (0,1,0) as those class rules, so **source order is the only thing
 * that decides the tie** — the same shape as U09's `@container` defect and as the reduced-motion
 * block above. Asserting the position is what stops a later child appending a `display` rule after
 * it and reintroducing the bug with every attribute assertion still green.
 */
describe('the platform’s own `hidden` wins in every sheet', () => {
  it.each([
    ['the mini toolbar', miniToolbarCss],
    ['the screentip', screentipCss],
    ['the toast', toastCss],
    ['progress', progressCss],
    ['the empty state', emptyStateCss],
  ])('%s restates it, and restates it last', (_name, sheet) => {
    const rule = '[hidden] { display: none; }';
    const at = sheet.indexOf(rule);
    expect(at, 'the sheet does not restate [hidden] at all').toBeGreaterThan(-1);
    // Nothing declares a `display` after it. A rule that did would win the tie and undo this.
    expect(sheet.slice(at + rule.length)).not.toContain('display:');
  });
});

// ── the timing table ─────────────────────────────────────────────────────────

describe('every span of time is a multiple of the one duration token', () => {
  it('names six spans, each with a positive multiple', () => {
    expect(feedbackTimingNames.length).toBe(6);
    for (const span of feedbackTimingNames) {
      expect(feedbackTimingMultiples[span], span).toBeGreaterThan(0);
    }
  });

  it('resolves each one from the generated token, and none of them to zero', () => {
    const base = Number.parseFloat(tokens.duration.transition);
    expect(base).toBeGreaterThan(0);
    for (const span of feedbackTimingNames) {
      expect(feedbackTimingMilliseconds(span)).toBe(base * feedbackTimingMultiples[span]);
      expect(feedbackTimingMilliseconds(span)).toBeGreaterThan(0);
    }
  });

  it('writes each one as a calc over the token and never as a number', () => {
    for (const span of feedbackTimingNames) {
      const value = feedbackTimingValue(span);
      expect(value).toBe(durationMultiple(feedbackTimingMultiples[span]));
      expect(value).toContain('var(--duration-transition)');
      expect(value).not.toMatch(/\d+m?s\b/);
    }
  });

  it('registers every one as a <time>, which is what makes it readable as a number', () => {
    for (const span of feedbackTimingNames) {
      expect(feedbackTimingCss).toContain(`@property ${feedbackTimingProperties[span]}`);
      expect(feedbackTimingCss).toContain(`${feedbackTimingProperties[span]}: ${feedbackTimingValue(span)};`);
    }
    expect(feedbackTimingCss.match(/syntax: '<time>'/g)?.length).toBe(feedbackTimingNames.length);
  });

  /**
   * The reader, and the reason it returns `undefined` rather than a number.
   *
   * A stub `getComputedStyle` is enough: what is being checked is that the *unit* is verified
   * rather than assumed, because `Number.parseFloat('calc(150ms * 4)')` returns `NaN` and
   * `Number.parseFloat('0.25rem')` returns `0.25` — a helper that trusted `parseFloat` would report
   * a plausible number for text it could not read, which is exactly the failure mode U07 named.
   */
  it.each([
    ['600ms', 600],
    ['0.6s', 600],
    ['-1s', -1000],
    ['calc(150ms * 4)', undefined],
    ['0.25rem', undefined],
    ['', undefined],
    ['600', undefined],
  ])('reads %s as %s', (raw, expected) => {
    const view = {
      getComputedStyle: () => ({ getPropertyValue: () => raw }),
    } as unknown as typeof globalThis;
    const previous = globalThis.getComputedStyle as unknown;
    (globalThis as { getComputedStyle?: unknown }).getComputedStyle = view.getComputedStyle;
    try {
      expect(resolveDurationMilliseconds({} as Element, '--anything')).toBe(expected);
    } finally {
      (globalThis as { getComputedStyle?: unknown }).getComputedStyle = previous;
    }
  });
});

// ── the vocabulary's own consistency ─────────────────────────────────────────

describe('the type roles this child wears', () => {
  it('the empty state’s heading is the display role, and it is the only one that is', () => {
    expect(feedbackTypeRoles.emptyHeading).toBe(typeRoleClass('display'));
    for (const [part, role] of Object.entries(feedbackTypeRoles)) {
      if (part === 'emptyHeading') continue;
      expect(role, `${part} wears the display role`).not.toBe(typeRoleClass('display'));
    }
  });

  /**
   * §4 of `DESIGN_TOKENS.md` as a fact about the tree rather than a sentence in a document:
   * *Young Serif is display-only. It belongs in empty states and onboarding, never in the chrome or
   * in document content.*
   *
   * The exception list is exactly two files — the module that *defines* the role, and the one that
   * hands it to the empty state. Anything else naming it is a component that has put a serif in the
   * chrome.
   */
  it('nothing else in src/ reaches for the display role', () => {
    const root = resolve(import.meta.dirname, '../src');
    const sources = sourcesUnder(root);
    expect(sources.length).toBeGreaterThan(30);
    const allowed = new Set([
      join(root, 'foundations', 'typography.ts'),
      join(root, 'feedback', 'feedback-model.ts'),
    ]);
    const offenders = sources.filter(
      (path) => !allowed.has(path) && /typeRoleClass\(\s*'display'\s*\)/.test(readFileSync(path, 'utf8')),
    );
    expect(offenders).toEqual([]);
  });
});

describe('nothing under src/feedback/ contains a colour', () => {
  const hex = /#[0-9a-fA-F]{3}(?:[0-9a-fA-F]{3})?\b/;

  /**
   * The stronger form of the literal-value lint, which `src/surfaces/` established: **no hex
   * anywhere at all**, comments included, where a linter cannot look. A tone's edge is a theme
   * member and a fill is a `var()`; there is nothing in this family a hex could legitimately be.
   */
  it('has no hex colour in any of its sources', () => {
    const root = resolve(import.meta.dirname, '../src/feedback');
    const sources = sourcesUnder(root);
    expect(sources.length).toBeGreaterThan(4);
    expect(sources.filter((path) => hex.test(readFileSync(path, 'utf8')))).toEqual([]);
  });

  it('and the grep can tell one from a token reference', () => {
    expect(hex.test("const edge = '#2e9e63';")).toBe(true);
    expect(hex.test('const edge = themeColor(scheme, tone.edge);')).toBe(false);
  });
});

describe('the empty state announces itself', () => {
  it('carries a live role rather than being a silent picture', () => {
    expect(emptyStateRole).toBe('status');
  });
});

/** Every `.ts` file under a directory, recursively. */
function sourcesUnder(directory: string): string[] {
  const found: string[] = [];
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) found.push(...sourcesUnder(path));
    else if (entry.endsWith('.ts')) found.push(path);
  }
  return found;
}
