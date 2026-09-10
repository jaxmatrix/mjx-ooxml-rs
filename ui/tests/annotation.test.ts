import { describe, expect, it } from 'vitest';

import {
  annotationIconSize,
  commentModels,
  connectorInsetUnits,
  firstOverlap,
  marginCardGapUnits,
  packMarginCards,
  packingCost,
  reviewPresentationAt,
  reviewSheetAtOrBelow,
  reviewWindow,
  stackMarginCards,
  trackedChangeKindNames,
  trackedChangeKinds,
  type MarginCard,
  type PackedCard,
  type PackingOptions,
} from '../src/annotation/annotation-model.ts';
import {
  assignAuthorColours,
  authorColourFor,
  authorColourMinimumDistance,
  authorColourSlots,
  authorColourVisibilityMinimum,
  authorInitials,
  authorKey,
} from '../src/annotation/author-colour.ts';
import {
  closestPairUnderVision,
  colourDistance,
  deltaE2000,
  deficiencyNames,
  naiveHashSlot,
  naiveHueRotation,
  seenAs,
  simulateDeficiency,
  visionNames,
  type Lab,
} from '../dev/colour-vision.ts';
import {
  manyAnnotations,
  sampleAuthorNames,
  tightlyClusteredAnchors,
  wellSpacedAnchors,
} from '../dev/annotation-fixtures.ts';
import { contrastRatio } from '../src/tokens/contrast.ts';
import { generatedDefault } from '../src/tokens/resolver.ts';
import { phoneShellAtOrBelow } from '../src/harness/presets.ts';
import { iconGlyphs } from '../src/icons/generated.ts';
import { iconId } from '../src/icons/manifest.ts';
import { rowsInWindow } from '../src/foundations/virtual-list.ts';
import { tokens } from '../tokens/tokens.ts';

/**
 * MJXOFF-193's gates.
 *
 * Two of them are the ticket's own traps and both are written the same way: **the naive answer is
 * computed beside the real one and asserted to differ.** A packing gate with no stack beside it is
 * a gate nobody has watched reject anything, and a colour-distinctness gate that never simulated a
 * deficiency is a gate that would pass a palette which merges into two colours.
 */

// ── the instrument, checked against somebody else's numbers ────────────────────────────────────

describe('CIEDE2000', () => {
  /**
   * Sharma, Wu & Dalal (2005), *"The CIEDE2000 color-difference formula"*, supplementary test data.
   *
   * These rows exist because the formula invites four specific mistakes, and each row is aimed at
   * one: the hue-angle wrap near 360°, the mean-hue wrap when two angles straddle it, the
   * blue-region rotation term, and the neutral case where a chroma of zero makes hue undefined.
   * An implementation that passes all nine has not merely been read carefully; it has been checked
   * against an outside party.
   */
  const sharma: readonly (readonly [Lab, Lab, number])[] = [
    [[50, 2.6772, -79.7751], [50, 0, -82.7485], 2.0425],
    [[50, 3.1571, -77.2803], [50, 0, -82.7485], 2.8615],
    [[50, 2.8361, -74.02], [50, 0, -82.7485], 3.4412],
    [[50, -1.3802, -84.2814], [50, 0, -82.7485], 1.0],
    [[50, -1.1848, -84.8006], [50, 0, -82.7485], 1.0],
    [[50, -0.9009, -85.5211], [50, 0, -82.7485], 1.0],
    [[50, 0, 0], [50, -1, 2], 2.3669],
    [[50, 2.49, -0.001], [50, -2.49, 0.0009], 7.1792],
    [[60.2574, -34.0099, 36.2677], [60.4626, -34.1751, 39.4387], 1.2644],
  ];

  it('agrees with the published test data on all nine rows', () => {
    for (const [first, second, expected] of sharma) {
      expect(deltaE2000(first, second)).toBeCloseTo(expected, 4);
    }
  });

  it('is symmetric and zero on identity', () => {
    for (const [first, second] of sharma) {
      expect(deltaE2000(first, second)).toBeCloseTo(deltaE2000(second, first), 10);
      expect(deltaE2000(first, first)).toBeCloseTo(0, 10);
    }
  });
});

describe('colour-vision simulation', () => {
  it('leaves normal vision alone', () => {
    for (const slot of authorColourSlots) {
      const colour = generatedDefault(slot.light);
      expect(seenAs(colour, 'normal')).toBe(colour);
    }
  });

  it('merges the pair a naive palette relies on', () => {
    // A saturated red and a saturated olive are far apart to a trichromat and identical to a
    // deuteranope. This is the mechanism the author palette is searched against, in one line.
    const red = '#ba2c2c';
    const olive = '#ba962c';
    expect(colourDistance(red, olive) ?? 0).toBeGreaterThan(8);
    const redSeen = simulateDeficiency(red, 'deuteranopia') ?? '';
    const oliveSeen = simulateDeficiency(olive, 'deuteranopia') ?? '';
    expect(colourDistance(redSeen, oliveSeen) ?? 99).toBeLessThan(1);
  });
});

// ── the author palette ─────────────────────────────────────────────────────────────────────────

const schemes = ['light', 'dark'] as const;

function slotColours(scheme: 'light' | 'dark'): string[] {
  return authorColourSlots.map((slot) => generatedDefault(scheme === 'light' ? slot.light : slot.dark));
}

describe('author colours', () => {
  it('offers eight slots, every one of them a token that exists', () => {
    expect(authorColourSlots).toHaveLength(8);
    for (const slot of authorColourSlots) {
      expect(generatedDefault(slot.light)).toMatch(/^#[0-9a-f]{6}$/i);
      expect(generatedDefault(slot.dark)).toMatch(/^#[0-9a-f]{6}$/i);
    }
  });

  for (const scheme of schemes) {
    it(`stays distinguishable in ${scheme} under every deficiency`, () => {
      const colours = slotColours(scheme);
      const closest = closestPairUnderVision(colours);
      expect(closest).toBeDefined();
      if (closest === undefined) return;
      // Named, so a failure says which two authors merged and to which reader.
      const message =
        `${authorColourSlots[closest.first]?.name ?? '?'} and ` +
        `${authorColourSlots[closest.second]?.name ?? '?'} are ${closest.distance.toFixed(2)} apart ` +
        `under ${closest.vision}`;
      expect(closest.distance, message).toBeGreaterThanOrEqual(authorColourMinimumDistance);
    });

    it(`sweeps all eight authors across all three deficiencies in ${scheme}`, () => {
      // The ticket asks for eight authors across the three common types; this is that sweep
      // written out, so the count is asserted rather than implied by a helper.
      const colours = slotColours(scheme);
      expect(colours).toHaveLength(8);
      expect(deficiencyNames).toHaveLength(3);
      let pairs = 0;
      for (const deficiency of deficiencyNames) {
        for (let i = 0; i < colours.length; i += 1) {
          for (let j = i + 1; j < colours.length; j += 1) {
            const a = seenAs(colours[i] ?? '', deficiency) ?? '';
            const b = seenAs(colours[j] ?? '', deficiency) ?? '';
            expect(colourDistance(a, b) ?? 0).toBeGreaterThanOrEqual(authorColourMinimumDistance);
            pairs += 1;
          }
        }
      }
      expect(pairs).toBe(3 * 28);
    });

    it(`is visible against the ${scheme} surface`, () => {
      const surface = tokens.theme[scheme].surface;
      for (const [index, colour] of slotColours(scheme).entries()) {
        const ratio = contrastRatio(colour, surface) ?? 0;
        expect(ratio, `${authorColourSlots[index]?.name ?? '?'} on ${scheme}`).toBeGreaterThanOrEqual(
          authorColourVisibilityMinimum,
        );
      }
    });
  }

  it('REJECTS the naive hue rotation, which passes to a normal-vision reader', () => {
    const naive = naiveHueRotation(8);
    // To a trichromat it is a better palette than the one this catalogue ships.
    let normalFloor = Number.POSITIVE_INFINITY;
    for (let i = 0; i < naive.length; i += 1) {
      for (let j = i + 1; j < naive.length; j += 1) {
        normalFloor = Math.min(normalFloor, colourDistance(naive[i] ?? '', naive[j] ?? '') ?? 0);
      }
    }
    expect(normalFloor).toBeGreaterThan(authorColourMinimumDistance);

    // …and under simulation it collapses. This is the assertion that fires if the shipped palette
    // is ever replaced by a generated one.
    const closest = closestPairUnderVision(naive);
    expect(closest).toBeDefined();
    expect(closest?.distance ?? 99).toBeLessThan(authorColourMinimumDistance);
    expect(closest?.vision).not.toBe('normal');
  });

  it('sweeps four visions, not three', () => {
    // Normal vision is in the sweep too: a palette separable only after simulation is not a palette.
    expect([...visionNames]).toEqual(['normal', 'protanopia', 'deuteranopia', 'tritanopia']);
  });
});

describe('author colour assignment', () => {
  it('gives the first eight authors eight different slots', () => {
    const roster = assignAuthorColours(sampleAuthorNames.slice(0, 8));
    expect(new Set(roster.values()).size).toBe(8);
  });

  it('wraps the ring at the ninth author rather than inventing a colour', () => {
    const roster = assignAuthorColours(sampleAuthorNames.slice(0, 9));
    expect(roster.get(authorKey(sampleAuthorNames[8] ?? ''))).toBe(0);
    expect(roster.get(authorKey(sampleAuthorNames[0] ?? ''))).toBe(0);
  });

  it('folds whitespace and case, because a document’s metadata does not', () => {
    const roster = assignAuthorColours(['Ada Lovelace', '  ada   lovelace ', 'Grace Hopper']);
    expect(roster.size).toBe(2);
    expect(authorColourFor('ADA LOVELACE', roster)).toBe(0);
  });

  it('REJECTS a hashed assignment: it collides in most real documents', () => {
    // The positive control for the *other* half of "stable per author". A hash is stable across
    // documents and useless inside one, and this is the measurement rather than the arithmetic.
    let collided = 0;
    const trials = sampleAuthorNames.length - 7;
    for (let start = 0; start < trials; start += 1) {
      const window = sampleAuthorNames.slice(start, start + 8);
      const slots = new Set(window.map((name) => naiveHashSlot(name, authorColourSlots.length)));
      if (slots.size < window.length) collided += 1;
    }
    expect(collided / trials).toBeGreaterThan(0.5);

    // …and the shipped assignment collides in none of the same windows.
    for (let start = 0; start < trials; start += 1) {
      const window = sampleAuthorNames.slice(start, start + 8);
      expect(new Set(assignAuthorColours(window).values()).size).toBe(8);
    }
  });

  it('takes initials from the name when the document carried none', () => {
    expect(authorInitials('Ada Lovelace')).toBe('AL');
    expect(authorInitials('Prince')).toBe('P');
    expect(authorInitials('Karen Spärck Jones')).toBe('KJ');
    expect(authorInitials('   ')).toBe('');
  });
});

// ── the packing problem ────────────────────────────────────────────────────────────────────────

const gap = 12;

function cardsOf(annotations: readonly { id: string; anchorTop: number }[], extent = 90): MarginCard[] {
  return annotations.map((entry) => ({ id: entry.id, anchorTop: entry.anchorTop, extent }));
}

/**
 * **The independent reference.** Nothing about it resembles pool-adjacent-violators.
 *
 * Every feasible layout has a set of *level sets* — maximal runs of cards that end up touching —
 * and there are only 2^(n−1) ways to cut an ordered list into runs. For each cut, a run that does
 * not contain the pinned card sits at its own mean (clamped into the column), and a run that does
 * contain it sits at the pin. Any cut whose runs come out in increasing order is a feasible layout;
 * the optimum is the cheapest of them, because the optimum's own level sets are one of the cuts.
 *
 * It is exponential and obviously correct, which is the trade a reference implementation exists to
 * make.
 */
function referenceOptimum(cards: readonly MarginCard[], options: PackingOptions): number[] {
  const ordered = [...cards]
    .map((card, index) => ({ card, index }))
    .sort((a, b) => a.card.anchorTop - b.card.anchorTop || a.index - b.index)
    .map((entry) => entry.card);
  const count = ordered.length;
  const offsets: number[] = [];
  let running = 0;
  for (const [index, card] of ordered.entries()) {
    offsets.push(running);
    running += card.extent + (index === count - 1 ? 0 : options.gap);
  }
  const wanted = ordered.map((card, index) => card.anchorTop - (offsets[index] ?? 0));
  const low = options.columnTop;
  const high =
    options.columnBottom === undefined ? Number.POSITIVE_INFINITY : options.columnBottom - running;
  const pinIndex =
    options.selected === undefined ? -1 : ordered.findIndex((card) => card.id === options.selected);
  const pin = pinIndex < 0 ? 0 : Math.min(high, Math.max(low, wanted[pinIndex] ?? low));

  let best: number[] | undefined;
  let bestCost = Number.POSITIVE_INFINITY;
  for (let mask = 0; mask < 1 << (count - 1); mask += 1) {
    const values: number[] = new Array<number>(count).fill(0);
    let start = 0;
    let feasible = true;
    let previous = Number.NEGATIVE_INFINITY;
    for (let cut = 0; cut < count; cut += 1) {
      const isLast = cut === count - 1;
      const cutHere = isLast || (mask & (1 << cut)) !== 0;
      if (!cutHere) continue;
      const block = wanted.slice(start, cut + 1);
      const holdsPin = pinIndex >= start && pinIndex <= cut;
      const mean = block.reduce((total, value) => total + value, 0) / block.length;
      const value = holdsPin ? pin : Math.min(high, Math.max(low, mean));
      if (value < previous - 1e-9) {
        feasible = false;
        break;
      }
      for (let i = start; i <= cut; i += 1) values[i] = value;
      previous = value;
      start = cut + 1;
    }
    if (!feasible) continue;
    const cost = values.reduce(
      (total, value, index) => total + (value - (wanted[index] ?? 0)) ** 2,
      0,
    );
    if (cost < bestCost) {
      bestCost = cost;
      best = values;
    }
  }
  if (best === undefined) throw new Error('no feasible layout, which cannot happen');
  return best.map((value, index) => value + (offsets[index] ?? 0));
}

function tops(placed: readonly PackedCard[]): number[] {
  return placed.map((card) => card.top);
}

describe('margin packing', () => {
  const options: PackingOptions = { gap, columnTop: 0 };
  const tight = cardsOf(tightlyClusteredAnchors);
  const spaced = cardsOf(wellSpacedAnchors);

  it('the tight fixture really is tight', () => {
    // A fixture assertion, and it earns its place: if a later child spreads these anchors out, the
    // packing gate silently stops testing packing and every assertion below still passes.
    const span = 388 - 300;
    const stackHeight = tight.length * 90 + (tight.length - 1) * gap;
    expect(tight.length).toBeGreaterThanOrEqual(7);
    expect(span).toBeLessThan(stackHeight / 4);
  });

  it('leaves no overlap', () => {
    const placed = packMarginCards(tight, options);
    expect(firstOverlap(placed, gap)).toBeUndefined();
  });

  it('places every card as near its anchor as packing allows', () => {
    const placed = packMarginCards(tight, options);
    const reference = referenceOptimum(tight, options);
    expect(tops(placed)).toHaveLength(reference.length);
    for (const [index, top] of tops(placed).entries()) {
      expect(top).toBeCloseTo(reference[index] ?? 0, 6);
    }
  });

  it('pulls cards UP, which is the whole difference from a stack', () => {
    const placed = packMarginCards(tight, options);
    expect(placed.some((card) => card.displacement < 0)).toBe(true);
    expect(stackMarginCards(tight, options).every((card) => card.displacement >= 0)).toBe(true);
  });

  it('holds the selected card at its anchor and makes the others yield', () => {
    for (const selected of ['c1', 'c2', 'c3']) {
      const withPin: PackingOptions = { ...options, selected };
      const placed = packMarginCards(tight, withPin);
      const pinned = placed.find((card) => card.id === selected);
      expect(pinned).toBeDefined();
      expect(pinned?.displacement ?? 1).toBeCloseTo(0, 6);
      expect(firstOverlap(placed, gap)).toBeUndefined();
      const reference = referenceOptimum(tight, withPin);
      for (const [index, top] of tops(placed).entries()) {
        expect(top).toBeCloseTo(reference[index] ?? 0, 6);
      }
      // The others yield: at least one card moved that the unpinned layout left alone.
      const free = packMarginCards(tight, options);
      expect(tops(placed)).not.toEqual(tops(free));
    }
  });

  it('lets the column’s own top win over the pin, and says so by moving it', () => {
    // ⚠ A real property and not a rounding case. Seven cards are about seven hundred pixels tall
    // and the last anchor is 388 into the document, so holding IT at its anchor would need six
    // cards above the column's own beginning. The pin yields to the bound, the layout is still the
    // optimum under both constraints, and everything above the pinned card ends up touching.
    const withPin: PackingOptions = { ...options, selected: 'r3' };
    const placed = packMarginCards(tight, withPin);
    const pinned = placed.find((card) => card.id === 'r3');
    expect(pinned?.displacement ?? 0).toBeGreaterThan(0);
    expect(Math.min(...tops(placed))).toBeCloseTo(options.columnTop, 6);
    expect(firstOverlap(placed, gap)).toBeUndefined();
    const reference = referenceOptimum(tight, withPin);
    for (const [index, top] of tops(placed).entries()) {
      expect(top).toBeCloseTo(reference[index] ?? 0, 6);
    }
  });

  it('a simple stack is worse — the positive control', () => {
    const packed = packMarginCards(tight, options);
    const stacked = stackMarginCards(tight, options);
    expect(firstOverlap(stacked, gap)).toBeUndefined();
    // It never overlaps, so an overlap-only gate would pass it. What it cannot do is be near.
    expect(packingCost(stacked)).toBeGreaterThan(packingCost(packed) * 1.5);
    const furthest = (cards: readonly PackedCard[]): number =>
      Math.max(...cards.map((card) => Math.abs(card.displacement)));
    expect(furthest(stacked)).toBeGreaterThan(furthest(packed));
  });

  it('a simple stack cannot hold the selection either', () => {
    const withPin: PackingOptions = { ...options, selected: 'r2' };
    const stacked = stackMarginCards(tight, withPin);
    const pinned = stacked.find((card) => card.id === 'r2');
    expect(Math.abs(pinned?.displacement ?? 0)).toBeGreaterThan(1);
  });

  it('AND is indistinguishable from the real one on well-spaced anchors', () => {
    // The trap, asserted. This is why the fixture above has to be tight: on the fixture a person
    // would naturally have written, the naive implementation is exactly right.
    expect(tops(packMarginCards(spaced, options))).toEqual(tops(stackMarginCards(spaced, options)));
  });

  it('respects the column’s top and bottom when it has one', () => {
    const bounded: PackingOptions = { gap, columnTop: 40, columnBottom: 900 };
    const placed = packMarginCards(tight, bounded);
    expect(firstOverlap(placed, gap)).toBeUndefined();
    expect(Math.min(...tops(placed))).toBeGreaterThanOrEqual(40 - 1e-9);
    const last = placed[placed.length - 1];
    expect((last?.top ?? 0) + (last?.extent ?? 0)).toBeLessThanOrEqual(900 + 1e-9);
    const reference = referenceOptimum(tight, bounded);
    for (const [index, top] of tops(placed).entries()) {
      expect(top).toBeCloseTo(reference[index] ?? 0, 6);
    }
  });

  it('is stable and total on the degenerate inputs', () => {
    expect(packMarginCards([], options)).toEqual([]);
    const single = packMarginCards(cardsOf([{ id: 'a', anchorTop: 500 }]), options);
    expect(single[0]?.top).toBe(500);
    const sameAnchor = packMarginCards(
      cardsOf([
        { id: 'a', anchorTop: 100 },
        { id: 'b', anchorTop: 100 },
      ]),
      options,
    );
    expect(sameAnchor.map((card) => card.id)).toEqual(['a', 'b']);
    expect(firstOverlap(sameAnchor, gap)).toBeUndefined();
  });

  it('the gap is a density multiple, never a length written here', () => {
    expect(marginCardGapUnits).toBeGreaterThan(0);
    expect(Number.isInteger(marginCardGapUnits)).toBe(true);
    expect(Number.isInteger(connectorInsetUnits)).toBe(true);
  });
});

// ── virtualisation ─────────────────────────────────────────────────────────────────────────────

describe('the review window', () => {
  const many = manyAnnotations(300);
  const placed = packMarginCards(cardsOf(many), { gap, columnTop: 0 });

  it('builds a bounded number of cards for three hundred annotations', () => {
    const viewport = 800;
    const built = rowsInWindow(reviewWindow(placed, 0, viewport));
    expect(placed).toHaveLength(300);
    // A ceiling satisfied by zero is no ceiling: both ends are asserted.
    expect(built).toBeGreaterThan(0);
    expect(built).toBeLessThan(20);
  });

  it('covers the viewport wherever the column is scrolled', () => {
    const viewport = 600;
    for (const offset of [0, 400, 4000, 12_000]) {
      const window_ = reviewWindow(placed, offset, viewport);
      const first = placed[window_.firstRow];
      const last = placed[window_.lastRow - 1];
      expect(first).toBeDefined();
      expect(last).toBeDefined();
      if (first === undefined || last === undefined) continue;
      expect(first.top).toBeLessThanOrEqual(Math.max(offset, placed[0]?.top ?? 0) + 1e-6);
      expect(last.top + last.extent).toBeGreaterThanOrEqual(
        Math.min(offset + viewport, (placed[placed.length - 1]?.top ?? 0) + (placed[placed.length - 1]?.extent ?? 0)) - 1e-6,
      );
    }
  });

  it('is empty for an empty column and never negative', () => {
    expect(rowsInWindow(reviewWindow([], 0, 500))).toBe(0);
    expect(rowsInWindow(reviewWindow(placed, -100, 0))).toBeGreaterThan(0);
  });
});

// ── the rest of the contract ───────────────────────────────────────────────────────────────────

describe('the two comment models', () => {
  it('differ in what they can do, not only in how they look', () => {
    expect(commentModels.legacy.repliable).toBe(false);
    expect(commentModels.legacy.resolvable).toBe(false);
    expect(commentModels.threaded.repliable).toBe(true);
    expect(commentModels.threaded.resolvable).toBe(true);
  });

  it('are told apart visually AND audibly', () => {
    expect(commentModels.legacy.presentation).not.toBe(commentModels.threaded.presentation);
    expect(commentModels.legacy.announcement).not.toBe(commentModels.threaded.announcement);
  });
});

describe('tracked changes', () => {
  it('carry four kinds, each with its own icon and verb', () => {
    expect(trackedChangeKindNames).toHaveLength(4);
    const icons = new Set(trackedChangeKindNames.map((kind) => trackedChangeKinds[kind].icon));
    const verbs = new Set(trackedChangeKindNames.map((kind) => trackedChangeKinds[kind].verb));
    expect(icons.size).toBe(4);
    expect(verbs.size).toBe(4);
  });

  it('name only icons the subset actually carries at the size a card draws them', () => {
    // Fluent draws each size separately, so an icon carried only at 20 renders NOTHING at 16 —
    // a blank space in a card, which is the failure icon.ts says nobody notices.
    for (const kind of trackedChangeKindNames) {
      const id = iconId(trackedChangeKinds[kind].icon, annotationIconSize, 'regular');
      expect(Object.keys(iconGlyphs), `${kind} icon`).toContain(id);
    }
    for (const name of ['comment', 'checkmark', 'delete', 'dismiss', 'chevron-down', 'chevron-up']) {
      expect(Object.keys(iconGlyphs), name).toContain(iconId(name, annotationIconSize, 'regular'));
    }
  });
});

describe('the two presentations', () => {
  it('alias the shell threshold rather than restating it', () => {
    expect(reviewSheetAtOrBelow).toBe(phoneShellAtOrBelow);
  });

  it('switch at the threshold, on the side a person would expect', () => {
    expect(reviewPresentationAt(reviewSheetAtOrBelow)).toBe('sheet');
    expect(reviewPresentationAt(reviewSheetAtOrBelow + 1)).toBe('margin');
    expect(reviewPresentationAt(390)).toBe('sheet');
    expect(reviewPresentationAt(834)).toBe('margin');
  });
});
