import { readdirSync, readFileSync, statSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, test } from 'vitest';

import {
  canvasReservedGestures,
  gestureInconsistencies,
  gestureNames,
  gestureRegionNames,
  gestureRegions,
  gesturesSuppressedBy,
  regionsTakingReservedGestures,
  reservedGestureException,
  reservedGestureSites,
  resolveGesture,
} from '../src/mobile/gesture-map.ts';
import {
  commandBarOrder,
  commandBarPartition,
  commandBarUnnameable,
  contextualActions,
  demotionCost,
  formFactorFor,
  isMobileFormFactor,
  isSelectionKind,
  landscapePhoneInlineAtOrBelow,
  largePhoneLandscapeViewport,
  largePhoneViewport,
  mobileFormFactorNames,
  mobileFormFactors,
  naiveCommandBarPartition,
  selectionKindNames,
  sharedSelectionCommands,
  shortViewportAtOrBelow,
  smallTabletAtOrBelow,
  smallTabletViewport,
  thumbReachThreshold,
  withinThumbReach,
  type MobileCommand,
} from '../src/mobile/mobile-model.ts';
import {
  defaultSheetDetent,
  detentFraction,
  detentNearest,
  isSheetDetent,
  projectFraction,
  sheetDetentNames,
  sheetDetents,
  sheetDismissBelowFraction,
  sheetDragClaim,
  snapDetent,
} from '../src/mobile/sheet-detents.ts';
import { sheetBoundaryFraction } from '../src/surfaces/surface-model.ts';
import {
  catalogueByTag,
  catalogueComponents,
  meetsTargetSize,
  mobileBarTargetMinimum,
} from '../src/mobile/touch-audit.ts';
import { accessibleHitTargetMinimum } from '../src/foundations/density.ts';
import { essentialCommandLimit, groupPriorityNames } from '../src/ribbon/ribbon-model.ts';
import { phoneShellAtOrBelow } from '../src/harness/presets.ts';
import { wordPhoneCommands } from '../stories/mobile/specimens.ts';

/**
 * MJXOFF-194's unit tier.
 *
 * Four things are proved here and none of them needs a browser: the **enumeration** of the
 * catalogue (which is the half of the touch audit that can fail before a page loads), the
 * **demotion order** against an independent implementation, the **gesture map's** internal
 * consistency, and the **nested-scroll rule** exhaustively.
 */

// ── the enumeration ──────────────────────────────────────────────────────────

const sourceRoot = resolve(import.meta.dirname, '../src');

interface FoundDefinition {
  readonly module: string;
  readonly definedAs: string;
}

function walk(directory: string, into: string[] = []): string[] {
  for (const entry of readdirSync(directory)) {
    const path = resolve(directory, entry);
    if (statSync(path).isDirectory()) walk(path, into);
    else if (entry.endsWith('.ts')) into.push(path);
  }
  return into;
}

/**
 * Every `customElements.define` in `src/`, with the **source text** of its first argument.
 *
 * ⚠ Half this catalogue registers through a `tags` constant rather than a literal, so a scanner
 * that only understood string literals would have silently skipped twenty-eight components — the
 * *found nothing* failure mode, inside the enumeration itself. Comparing expression text handles
 * both spellings and resolves nothing. `the scanner sees both spellings` below is the instrument
 * test that keeps that true.
 */
function definitionsInSource(): FoundDefinition[] {
  const found: FoundDefinition[] = [];
  for (const path of walk(sourceRoot)) {
    const module = path.slice(sourceRoot.length + 1).replaceAll('\\', '/');
    const text = readFileSync(path, 'utf8');
    for (const match of text.matchAll(/customElements\.define\(\s*([^,]+?)\s*,/g)) {
      const expression = match[1];
      if (expression === undefined) continue;
      found.push({ module, definedAs: expression.trim() });
    }
  }
  return found;
}

function key(definition: FoundDefinition): string {
  return `${definition.module} :: ${definition.definedAs}`;
}

describe('the catalogue enumerates itself', () => {
  test('the scanner sees both spellings, and is not looking at nothing', () => {
    const found = definitionsInSource();
    // Anti-vacuity, in both directions. A scanner that had stopped matching would report zero and
    // every set comparison below would pass by being empty on both sides.
    expect(found.length).toBeGreaterThan(40);
    expect(found.some((definition) => definition.definedAs.startsWith("'"))).toBe(true);
    expect(found.some((definition) => !definition.definedAs.startsWith("'"))).toBe(true);
  });

  test('every component the source defines is in the registry, and nothing else is', () => {
    const found = definitionsInSource().map(key).sort();
    const declared = catalogueComponents.map((component) => key(component)).sort();

    const missing = found.filter((entry) => !declared.includes(entry));
    const stale = declared.filter((entry) => !found.includes(entry));

    expect(
      missing,
      'These custom elements are defined in src/ and are not in catalogueComponents, so the ' +
        'touch-target sweep does not know they exist. Add each one to ' +
        'src/mobile/touch-audit.ts with its audit kind.\n  ' +
        missing.join('\n  '),
    ).toEqual([]);
    expect(
      stale,
      'These registry entries name a definition that is no longer in src/. Either the module ' +
        'moved or the element went away.\n  ' + stale.join('\n  '),
    ).toEqual([]);
    expect(found).toEqual(declared);
  });

  test('every entry that claims to have no targets says why', () => {
    for (const component of catalogueComponents) {
      if (component.kind === 'targets') continue;
      expect(
        component.reason ?? '',
        `${component.tag} is declared ${component.kind} and gives no reason. A component ` +
          'excused from the audit has to say what it is instead.',
      ).not.toBe('');
    }
  });

  test('a tag is spelled once', () => {
    expect(catalogueByTag.size).toBe(catalogueComponents.length);
    for (const component of catalogueComponents) {
      expect(component.tag).toMatch(/^mjx-[a-z-]+$/);
    }
  });
});

// ── the target-size rule ─────────────────────────────────────────────────────

describe('WCAG 2.2 target size, as the rule rather than a simplification', () => {
  const big = { x: 0, y: 0, width: 40, height: 40 };
  const thin = { x: 0, y: 0, width: 8, height: 200 };

  test('a target at or over the floor on both axes passes by size', () => {
    expect(meetsTargetSize(big, [])).toEqual({ ok: true, rule: 'size' });
    expect(meetsTargetSize({ x: 0, y: 0, width: 24, height: 24 }, [])).toEqual({
      ok: true,
      rule: 'size',
    });
  });

  test('BOTH axes are asserted, which is this child’s own decision', () => {
    const wide = { x: 0, y: 0, width: 200, height: 12 };
    const tall = { x: 0, y: 0, width: 12, height: 200 };
    const crowdingWide = [{ x: 0, y: 20, width: 200, height: 12 }];
    const crowdingTall = [{ x: 20, y: 0, width: 12, height: 200 }];
    // MJXOFF-193 flagged that the catalogue asserts the block axis only. A 200 x 12 target and a
    // 12 x 200 one are equally unhittable, and only a both-axes rule says so about both.
    expect(meetsTargetSize(wide, crowdingWide).ok).toBe(false);
    expect(meetsTargetSize(tall, crowdingTall).ok).toBe(false);
  });

  test('a thin target with nothing near it passes by spacing', () => {
    expect(meetsTargetSize(thin, []).rule).toBe('spacing');
    expect(meetsTargetSize(thin, [{ x: 200, y: 0, width: 8, height: 200 }]).rule).toBe('spacing');
  });

  test('a thin target with a neighbour inside the circle fails, and says how near', () => {
    // The neighbour has to be near the target's CENTRE, not near its edge — the criterion is about
    // two circles of the minimum diameter, and a long thin control's own extent is not what decides.
    const verdict = meetsTargetSize(thin, [{ x: 10, y: 90, width: 8, height: 20 }]);
    expect(verdict.ok).toBe(false);
    expect(verdict.ok === false ? verdict.nearest : Number.NaN).toBeLessThan(
      accessibleHitTargetMinimum,
    );
  });

  test('the intersection test is a centre distance, and the boundary is exact', () => {
    const target = { x: 0, y: 0, width: 8, height: 8 };
    const justClear = [{ x: accessibleHitTargetMinimum, y: 0, width: 8, height: 8 }];
    const justInside = [{ x: accessibleHitTargetMinimum - 1, y: 0, width: 8, height: 8 }];
    expect(meetsTargetSize(target, justClear).ok).toBe(true);
    expect(meetsTargetSize(target, justInside).ok).toBe(false);
  });

  test('the mobile bars are held higher than the catalogue floor', () => {
    expect(mobileBarTargetMinimum).toBeGreaterThan(accessibleHitTargetMinimum);
    expect(meetsTargetSize({ x: 0, y: 0, width: 32, height: 32 }, [], mobileBarTargetMinimum).ok).toBe(
      // 32 clears the catalogue floor and not the mobile one, which is the whole reason there are
      // two numbers.
      true,
    );
    expect(
      meetsTargetSize(
        { x: 0, y: 0, width: 32, height: 32 },
        [{ x: 20, y: 0, width: 32, height: 32 }],
        mobileBarTargetMinimum,
      ).ok,
    ).toBe(false);
  });
});

// ── the form factors ─────────────────────────────────────────────────────────

describe('the form factors', () => {
  test('the three judged shapes land where the presentations say they do', () => {
    expect(formFactorFor(largePhoneViewport)).toBe('phonePortrait');
    expect(formFactorFor(largePhoneLandscapeViewport)).toBe('phoneLandscape');
    expect(formFactorFor(smallTabletViewport)).toBe('smallTablet');
    expect(formFactorFor({ inline: 1440, block: 900 })).toBe('desktop');
  });

  test('a landscape phone is NOT a tablet, which is the whole reason the media half exists', () => {
    // 844 x 390: wider than the phone threshold, so by width alone it is a small tablet.
    const landscape = { inline: 844, block: 390 };
    expect(landscape.inline).toBeGreaterThan(phoneShellAtOrBelow);
    expect(formFactorFor({ inline: landscape.inline, block: 1000 })).toBe('smallTablet');
    expect(formFactorFor(landscape)).toBe('phoneLandscape');
  });

  test('a short DESKTOP window is not a landscape phone', () => {
    // A 1440 x 420 window is somebody with a very short browser, not a phone on its side, and the
    // landscape presentation would put a phone bar on a desktop.
    expect(formFactorFor({ inline: 1440, block: 420 })).toBe('desktop');
    expect(formFactorFor({ inline: landscapePhoneInlineAtOrBelow + 1, block: 420 })).toBe('desktop');
  });

  test('THE BIGGEST PHONE ON THE MARKET is not a desktop, on its side', () => {
    // 932 x 430 is an iPhone 15 Pro Max in landscape, and it is WIDER than the small-tablet
    // threshold. The first draft of formFactorFor bounded the landscape branch by that threshold
    // and sent this device to the desktop presentation — the one presentation a phone must never
    // get. That defect is why `landscapePhoneInlineAtOrBelow` exists.
    expect(largePhoneLandscapeViewport.inline).toBeGreaterThan(smallTabletAtOrBelow);
    expect(formFactorFor(largePhoneLandscapeViewport)).toBe('phoneLandscape');
  });

  test('a narrow short viewport is a portrait phone, not a landscape one', () => {
    expect(formFactorFor({ inline: 390, block: 390 })).toBe('phonePortrait');
  });

  test('the thresholds are ordered, and the harness presets land informatively', () => {
    expect(phoneShellAtOrBelow).toBeLessThan(smallTabletAtOrBelow);
    expect(shortViewportAtOrBelow).toBeLessThan(phoneShellAtOrBelow);
    expect(formFactorFor({ inline: 390, block: 844 })).toBe('phonePortrait');
    expect(formFactorFor({ inline: 834, block: 1112 })).toBe('smallTablet');
    expect(formFactorFor({ inline: 1440, block: 900 })).toBe('desktop');
  });

  test('every form factor is named and has a spec', () => {
    for (const factor of mobileFormFactorNames) {
      expect(isMobileFormFactor(factor)).toBe(true);
      expect(mobileFormFactors[factor].visibleSlots).toBeGreaterThan(0);
      expect(mobileFormFactors[factor].description).not.toBe('');
    }
    expect(isMobileFormFactor('phone')).toBe(false);
  });
});

// ── reachability ─────────────────────────────────────────────────────────────

describe('reachability', () => {
  test('the band is the bottom of the viewport, and a target must lie wholly inside it', () => {
    const threshold = thumbReachThreshold(largePhoneViewport);
    expect(withinThumbReach({ top: threshold, bottom: threshold + 40 }, largePhoneViewport)).toBe(
      true,
    );
    expect(withinThumbReach({ top: threshold - 1, bottom: threshold + 40 }, largePhoneViewport)).toBe(
      false,
    );
  });

  test('half a target inside the band is not inside the band', () => {
    const threshold = thumbReachThreshold(largePhoneViewport);
    // The half a person can reach is the half nearest the edge of the target, which is exactly the
    // half they will miss.
    expect(withinThumbReach({ top: threshold - 20, bottom: threshold + 20 }, largePhoneViewport)).toBe(
      false,
    );
  });

  test('landscape has a tighter band in absolute pixels, which is why it gets less padding', () => {
    expect(thumbReachThreshold(largePhoneLandscapeViewport)).toBeLessThan(
      thumbReachThreshold(largePhoneViewport),
    );
  });
});

// ── the demotion order ───────────────────────────────────────────────────────

/**
 * An **independent** implementation of the order — a selection sort over an explicit key, with
 * nothing in common with `commandBarOrder`'s comparator-plus-index-tiebreak.
 *
 * MJXOFF-193's standard: the gate that matters is the one comparing against an answer computed a
 * different way, because a gate that re-runs the implementation is asking it to grade its own
 * homework.
 */
function independentOrder(commands: readonly MobileCommand[]): MobileCommand[] {
  const remaining = commands.map((command, index) => ({ command, index }));
  const ordered: MobileCommand[] = [];
  while (remaining.length > 0) {
    let best = 0;
    for (let i = 1; i < remaining.length; i += 1) {
      const candidate = remaining[i];
      const incumbent = remaining[best];
      if (candidate === undefined || incumbent === undefined) continue;
      const candidateRank = groupPriorityNames.indexOf(candidate.command.priority);
      const incumbentRank = groupPriorityNames.indexOf(incumbent.command.priority);
      if (candidateRank !== incumbentRank) {
        if (candidateRank < incumbentRank) best = i;
        continue;
      }
      if (candidate.command.essential !== incumbent.command.essential) {
        if (candidate.command.essential) best = i;
        continue;
      }
      if (candidate.index < incumbent.index) best = i;
    }
    const chosen = remaining.splice(best, 1)[0];
    if (chosen !== undefined) ordered.push(chosen.command);
  }
  return ordered;
}

describe('the command bar follows U04’s ladder', () => {
  test('the fixture is DISCRIMINATING: declaration order is not ladder order', () => {
    // Without this, every assertion below would be satisfied by an implementation that ignored the
    // ladder entirely. MJXOFF-193's `the tight fixture really is tight`, met a second time.
    expect(commandBarOrder(wordPhoneCommands).map((command) => command.id)).not.toEqual(
      wordPhoneCommands.map((command) => command.id),
    );
  });

  test('the order equals an independently computed one', () => {
    expect(commandBarOrder(wordPhoneCommands).map((command) => command.id)).toEqual(
      independentOrder(wordPhoneCommands).map((command) => command.id),
    );
  });

  test('nothing is ever removed, at every slot count', () => {
    for (let slots = 0; slots <= wordPhoneCommands.length + 2; slots += 1) {
      const { visible, overflow } = commandBarPartition(wordPhoneCommands, slots);
      const seen = [...visible, ...overflow].map((command) => command.id).sort();
      expect(seen).toEqual(wordPhoneCommands.map((command) => command.id).sort());
    }
  });

  test('a command that opens a popup is never in the visible run', () => {
    for (const factor of mobileFormFactorNames) {
      const { visible } = commandBarPartition(
        wordPhoneCommands,
        mobileFormFactors[factor].visibleSlots,
      );
      expect(visible.filter((command) => command.hasPopup)).toEqual([]);
    }
    // And the fixture actually contains some, or the assertion above is about nothing.
    expect(wordPhoneCommands.some((command) => command.hasPopup)).toBe(true);
  });

  test('at most three commands from one group survive, and the fixture makes that bite', () => {
    const { visible } = commandBarPartition(wordPhoneCommands, wordPhoneCommands.length);
    const counts = new Map<string, number>();
    for (const command of visible) counts.set(command.group, (counts.get(command.group) ?? 0) + 1);
    for (const [group, count] of counts) {
      expect(count, `${group} kept ${String(count)} commands`).toBeLessThanOrEqual(
        essentialCommandLimit,
      );
    }
    // Font declares four essential commands, so the ceiling has something to refuse.
    const font = wordPhoneCommands.filter(
      (command) => command.group === 'Font' && command.essential,
    );
    expect(font.length).toBeGreaterThan(essentialCommandLimit);
  });

  test('every command has an icon and a name', () => {
    expect(commandBarUnnameable(wordPhoneCommands)).toEqual([]);
    expect(
      commandBarUnnameable([
        { ...(wordPhoneCommands[0] as MobileCommand), icon: '' },
      ]),
    ).toHaveLength(1);
  });

  test('the ladder’s answer costs nothing, and taking the first N costs more', () => {
    for (const factor of mobileFormFactorNames) {
      const slots = mobileFormFactors[factor].visibleSlots;
      const real = commandBarPartition(wordPhoneCommands, slots);
      const naive = naiveCommandBarPartition(wordPhoneCommands, slots);
      expect(demotionCost(real)).toBe(0);
      expect(
        demotionCost(naive),
        `at ${factor} the naive partition is no worse, which means the fixture cannot tell the ` +
          'two apart and this whole comparison is vacuous',
      ).toBeGreaterThan(0);
    }
  });

  test('demotionCost is a measurement rather than a verdict', () => {
    // A pair the wrong way round costs one; two cost two. A cost function that only ever returned
    // zero or one would make *worse* unprovable.
    // Three commands from three priorities, none of which carries a popup — a popup in the visible
    // run is a rule break and costs one of its own, which would confuse a test about ORDER.
    const plain = wordPhoneCommands.filter((command) => !command.hasPopup);
    const primary = plain.find((command) => command.priority === 'primary');
    const ancillary = plain.find((command) => command.priority === 'ancillary');
    const secondary = plain.find((command) => command.priority === 'secondary');
    if (primary === undefined || ancillary === undefined || secondary === undefined) {
      throw new Error('the fixture no longer spans three priorities');
    }
    expect(demotionCost({ visible: [primary, ancillary], overflow: [] })).toBe(0);
    expect(demotionCost({ visible: [ancillary, primary], overflow: [] })).toBe(1);
    expect(demotionCost({ visible: [ancillary, secondary, primary], overflow: [] })).toBe(3);
  });
});

describe('the contextual action bar', () => {
  test('every selection but none offers the shared four, first', () => {
    for (const kind of selectionKindNames) {
      if (kind === 'none') continue;
      const actions = contextualActions(kind);
      expect(actions.slice(0, sharedSelectionCommands.length).map((command) => command.id)).toEqual(
        sharedSelectionCommands.map((command) => command.id),
      );
    }
  });

  test('no selection is no actions at all, which is what makes it no bar', () => {
    expect(contextualActions('none')).toEqual([]);
  });

  test('every action carries an icon and a name, so a bar with no labels is still readable', () => {
    for (const kind of selectionKindNames) {
      expect(commandBarUnnameable(contextualActions(kind))).toEqual([]);
    }
  });

  test('the kinds are recognised and nothing else is', () => {
    for (const kind of selectionKindNames) expect(isSelectionKind(kind)).toBe(true);
    expect(isSelectionKind('picture')).toBe(false);
  });

  test('ids are unique inside a selection, so nothing is offered twice', () => {
    for (const kind of selectionKindNames) {
      const ids = contextualActions(kind).map((command) => command.id);
      expect(new Set(ids).size).toBe(ids.length);
    }
  });
});

// ── the gesture map ──────────────────────────────────────────────────────────

describe('the gesture map', () => {
  test('every region’s claims equal what its touch-action actually takes', () => {
    const inconsistent = gestureInconsistencies();
    expect(
      inconsistent,
      'A region that claims one gesture and writes a touch-action taking three has quietly taken ' +
        'two more from the document:\n  ' +
        inconsistent
          .map(
            (entry) =>
              `${entry.region}: declared ${entry.declared.join('/')} , derived ${entry.derived.join('/')}`,
          )
          .join('\n  '),
    ).toEqual([]);
  });

  test('the derivation is not vacuous', () => {
    expect(gesturesSuppressedBy('auto')).toEqual([]);
    expect(gesturesSuppressedBy('pan-x')).toEqual(['panInline']);
    expect(gesturesSuppressedBy('pan-y')).toEqual(['panBlock']);
    expect(gesturesSuppressedBy('none')).toContain('pinch');
    expect(gesturesSuppressedBy('none')).toHaveLength(4);
  });

  test('exactly one region takes a gesture the canvas reserves, and it is the named one', () => {
    expect(regionsTakingReservedGestures()).toEqual([reservedGestureException]);
  });

  test('the canvas region claims nothing', () => {
    expect(gestureRegions.canvas.claims).toEqual([]);
    for (const gesture of gestureNames) {
      expect(resolveGesture(gesture, 'canvas')).toBe('canvas');
    }
  });

  test('AMBIGUITY RESOLVES TO THE CANVAS — every unclaimed pair, in both directions', () => {
    for (const gesture of gestureNames) {
      for (const region of gestureRegionNames) {
        const claimed = gestureRegions[region].claims.includes(gesture);
        expect(resolveGesture(gesture, region)).toBe(claimed ? 'chrome' : 'canvas');
      }
    }
  });

  test('tap, double-tap and long-press are nobody’s, deliberately', () => {
    // touch-action cannot express them, so a claim on one would be a claim this table could not
    // enforce. Saying so as a test is what stops a later child adding one.
    for (const gesture of ['tap', 'doubleTap', 'longPress'] as const) {
      for (const region of gestureRegionNames) {
        expect(resolveGesture(gesture, region)).toBe('canvas');
      }
    }
  });

  test('pinch survives everywhere but the grab handle', () => {
    for (const region of gestureRegionNames) {
      if (region === reservedGestureException) continue;
      expect(resolveGesture('pinch', region)).toBe('canvas');
      expect(resolveGesture('twoFingerPan', region)).toBe('canvas');
    }
    expect(canvasReservedGestures).toEqual(['pinch', 'twoFingerPan']);
  });

  test('every region says what it does and what it leaves', () => {
    for (const region of gestureRegionNames) {
      expect(gestureRegions[region].does).not.toBe('');
      expect(gestureRegions[region].leaves).not.toBe('');
    }
  });

  test('EVERY PLACE IN THE CATALOGUE that takes the document’s gestures is written down', () => {
    // ⚠ The region table describes the mobile shell, so a browser gate over the mobile shell's own
    // stories checks the mobile shell — a sweep over what this child happened to touch, which is
    // the trap the ticket names one level up. This scan is catalogue-wide, costs nothing, and is
    // what found the five pre-existing sites nobody had written down.
    const found = new Set<string>();
    let declarations = 0;
    for (const path of walk(sourceRoot)) {
      const module = path.slice(sourceRoot.length + 1).replaceAll('\\', '/');
      // Comments first: this file's own prose explains the rule in the words it forbids.
      const text = readFileSync(path, 'utf8')
        .replaceAll(/\/\*[\s\S]*?\*\//g, ' ')
        .replaceAll(/(^|\s)\/\/[^\n]*/g, '$1 ');
      for (const match of text.matchAll(/touch-action:\s*([^;\n]+);/g)) {
        declarations += 1;
        const value = (match[1] ?? '').trim();
        if (value === 'none') {
          found.add(module);
          continue;
        }
        const named = /gestureRegions\.(\w+)\.touchAction/.exec(value);
        const region = named?.[1];
        if (
          region !== undefined &&
          (gestureRegionNames as readonly string[]).includes(region) &&
          gestureRegions[region as (typeof gestureRegionNames)[number]].touchAction === 'none'
        ) {
          found.add(module);
        }
      }
    }

    // Anti-vacuity: a scanner that had stopped matching would report an empty set, and an empty set
    // equals an empty set only if the declared list were also empty — which it is not, so the
    // comparison below would fail. This is the belt as well as the braces.
    expect(declarations).toBeGreaterThan(5);

    const declared = reservedGestureSites.map((site) => site.module).sort();
    expect(
      [...found].sort(),
      'A component writes touch-action: none and is not in reservedGestureSites, or a site in the ' +
        'list no longer writes it. Every place that takes pinch away from the document has to say ' +
        'why, in one list.',
    ).toEqual(declared);

    for (const site of reservedGestureSites) {
      expect(site.what).not.toBe('');
      expect(site.because).not.toBe('');
    }
  });
});

// ── the sheet ────────────────────────────────────────────────────────────────

describe('the sheet’s detents', () => {
  test('full is an ALIAS of U09’s boundary fraction, not a fourth number', () => {
    expect(sheetDetents.full.fraction).toBe(sheetBoundaryFraction);
  });

  test('the detents are ordered and inside the boundary', () => {
    const fractions = sheetDetentNames.map((name) => sheetDetents[name].fraction);
    expect([...fractions].sort((a, b) => a - b)).toEqual(fractions);
    for (const fraction of fractions) {
      expect(fraction).toBeGreaterThan(sheetDismissBelowFraction);
      expect(fraction).toBeLessThanOrEqual(sheetBoundaryFraction);
    }
    expect(isSheetDetent(defaultSheetDetent)).toBe(true);
    expect(isSheetDetent('tall')).toBe(false);
  });

  test('a still release snaps to the nearest detent', () => {
    for (const name of sheetDetentNames) {
      expect(snapDetent({ fraction: sheetDetents[name].fraction, velocity: 0 })).toEqual({
        kind: 'snap',
        detent: name,
      });
      expect(detentFraction(name)).toBe(sheetDetents[name].fraction);
      expect(detentNearest(sheetDetents[name].fraction)).toBe(name);
    }
  });

  test('a flick is PROJECTED before it is rounded, which is what makes it a flick', () => {
    const from = sheetDetents.full.fraction;
    // Slow: stays where it is.
    expect(snapDetent({ fraction: from, velocity: 0 })).toEqual({ kind: 'snap', detent: 'full' });
    // Fast downward: goes past half. Without the projection this would round to full.
    const fast = { fraction: from, velocity: -0.003 };
    expect(projectFraction(fast)).toBeLessThan(sheetDetents.half.fraction);
    expect(snapDetent(fast)).toEqual({ kind: 'snap', detent: 'peek' });
  });

  test('a release below the dismissal threshold dismisses, and one just above does not', () => {
    expect(snapDetent({ fraction: sheetDismissBelowFraction - 0.01, velocity: 0 })).toEqual({
      kind: 'dismiss',
    });
    expect(snapDetent({ fraction: sheetDismissBelowFraction + 0.01, velocity: 0 }).kind).toBe(
      'snap',
    );
  });

  test('a slow drag that stops just above the threshold stays; a flick from halfway goes', () => {
    // The two cases only the projection tells apart, which is why the dismissal test is applied to
    // the projected fraction rather than to the released one.
    expect(snapDetent({ fraction: 0.2, velocity: 0 }).kind).toBe('snap');
    expect(snapDetent({ fraction: sheetDetents.half.fraction, velocity: -0.005 })).toEqual({
      kind: 'dismiss',
    });
  });

  test('the dismissal threshold is well under peek, because the two mistakes cost differently', () => {
    expect(sheetDismissBelowFraction).toBeLessThan(sheetDetents.peek.fraction * 0.75);
  });
});

describe('THE NESTED-SCROLL RULE — the classic mobile defect, exhaustively', () => {
  const directions = [-40, -1, 0, 1, 40];
  const scrolls = [0, 1, 250];

  test('the handle and the header always win, whatever the content is doing', () => {
    for (const scrollTop of scrolls) {
      for (const deltaBlock of directions) {
        expect(sheetDragClaim({ onHandle: true, onHeader: false, scrollTop, deltaBlock })).toBe(
          'sheet',
        );
        expect(sheetDragClaim({ onHandle: false, onHeader: true, scrollTop, deltaBlock })).toBe(
          'sheet',
        );
      }
    }
  });

  test('A SCROLLED SCROLLER KEEPS THE DRAG, in either direction', () => {
    // This is the rule whose absence dismisses a sheet when a person flicks a long list, and it is
    // the one this whole file exists for.
    for (const deltaBlock of directions) {
      expect(sheetDragClaim({ onHandle: false, onHeader: false, scrollTop: 1, deltaBlock })).toBe(
        'content',
      );
      expect(sheetDragClaim({ onHandle: false, onHeader: false, scrollTop: 250, deltaBlock })).toBe(
        'content',
      );
    }
  });

  test('at the top of the scroller, a downward drag hands off and an upward one does not', () => {
    expect(sheetDragClaim({ onHandle: false, onHeader: false, scrollTop: 0, deltaBlock: 40 })).toBe(
      'sheet',
    );
    expect(sheetDragClaim({ onHandle: false, onHeader: false, scrollTop: 0, deltaBlock: -40 })).toBe(
      'content',
    );
    // Exactly zero is not a direction yet, so it stays with the content until it becomes one.
    expect(sheetDragClaim({ onHandle: false, onHeader: false, scrollTop: 0, deltaBlock: 0 })).toBe(
      'content',
    );
  });

  test('the truth table is complete and has both answers in it', () => {
    const answers = new Set<string>();
    for (const onHandle of [false, true]) {
      for (const onHeader of [false, true]) {
        for (const scrollTop of scrolls) {
          for (const deltaBlock of directions) {
            answers.add(sheetDragClaim({ onHandle, onHeader, scrollTop, deltaBlock }));
          }
        }
      }
    }
    expect([...answers].sort()).toEqual(['content', 'sheet']);
  });
});
