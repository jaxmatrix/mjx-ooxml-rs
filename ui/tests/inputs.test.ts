import { describe, expect, test } from 'vitest';

import { tokens, type ColorScheme } from '../tokens/tokens.ts';
import { bodyTextMinimum, contrastRatio, nonTextMinimum } from '../dev/contrast.ts';
import {
  controlStateNames,
  controlStateSpecs,
  controlStatesCss,
  effectiveStatePaint,
  resolvedStateFingerprint,
  type ControlStateSpec,
  type EffectiveStatePaint,
} from '../src/controls/control-states.ts';
import { accessibleHitTargetMinimum } from '../src/foundations/density.ts';
import { cellsInWindow, galleryRowPlan, galleryWindow } from '../src/gallery/gallery-model.ts';
import { iconRequests } from '../src/icons/manifest.ts';
import {
  applySliderAction,
  boxGlyphs,
  boxStateCascade,
  boxStateNames,
  boxStates,
  boxStatesCss,
  boxPaint,
  composeStatePaint,
  decimalPlaces,
  defaultPageStep,
  displayTextFor,
  fieldPaint,
  fieldStateCascade,
  fieldStateNames,
  fieldStates,
  fieldStatesCss,
  filterOptions,
  inputBaseCss,
  inputFocusPattern,
  inputFocusPatterns,
  inputTags,
  listboxKeyAction,
  listboxPageRows,
  nextOptionIndex,
  optionCss,
  optionPaint,
  optionStateCascade,
  optionStateNames,
  optionStateOf,
  optionStates,
  optionStatesCss,
  optionSecondaryTextIsSizeNotColour,
  paintSpecOf,
  resolvePaintFingerprint,
  resolvedBoxFingerprint,
  resolvedFieldFingerprint,
  resolvedOptionFingerprint,
  roundTo,
  sliderCss,
  sliderFraction,
  sliderIndicatorPairs,
  sliderKeyAction,
  sliderLabelBackground,
  segmentedSheet,
  sliderPaints,
  snapToStep,
  tabStopsPerControl,
  typeaheadIndex,
  valueAtFraction,
  type DerivedStateSpec,
  type OptionDescriptor,
  type SliderRange,
} from '../src/inputs/input-model.ts';
import {
  clampMeasure,
  decimalCharacter,
  formatMeasure,
  measureUnitNames,
  measureFailureMessages,
  measureUnits,
  parseMeasure,
  pointsFrom,
  pointsIn,
  roundToPrecision,
  stepMeasure,
  unitFromSpelling,
  withinRange,
  type MeasureUnit,
} from '../src/inputs/measure.ts';


/**
 * The input model, checked before a browser is involved.
 *
 * `tests/browser/inputs.spec.ts` measures what a browser computed and drives real key presses,
 * which is the claim that matters. This tier proves what a browser is a slow and indirect way of
 * asking about:
 *
 * * **the arithmetic**, which has correct answers and should be tested as arithmetic — the slider's
 *   snapping and both of its ends, and every unit conversion the measure grammar performs;
 * * **the tables are internally distinct, internally legible and internally *indicated*** in both
 *   schemes, so a palette re-seed that collapses two states or drops an indicator below 3 : 1 is
 *   caught in a second;
 * * **the near-duplication of the composition machinery is not a duplication**, by asserting it
 *   equals the function it nearly duplicates over every row of the shared table;
 * * **the stylesheets cannot acquire the specificity accident** MJXOFF-183 shipped, for the fifth
 *   child running.
 *
 * ⚠ No colour, size or duration is written in this file. Every expectation is derived from the
 * generated tokens or from the model itself.
 */

const schemes: readonly ColorScheme[] = ['light', 'dark'];

function hexOf(member: string | undefined, scheme: ColorScheme): string | undefined {
  if (member === undefined || member === 'transparent') return undefined;
  return tokens.theme[scheme][member as keyof (typeof tokens.theme)['light']];
}

function ratio(front: string | undefined, back: string | undefined, scheme: ColorScheme): number {
  const a = hexOf(front, scheme);
  const b = hexOf(back, scheme);
  if (a === undefined || b === undefined) return Number.POSITIVE_INFINITY;
  return contrastRatio(a, b) ?? Number.POSITIVE_INFINITY;
}

// ── the shared composition machinery is shared, not copied ───────────────────

describe('the composition machinery', () => {
  test('composeStatePaint equals effectiveStatePaint for every row of the control table', () => {
    for (const state of controlStateNames) {
      expect(
        composeStatePaint(controlStateSpecs[state], controlStateSpecs.rest),
        `${state} composes differently here than in control-states.ts`,
      ).toEqual(effectiveStatePaint(state));
    }
  });

  test('resolvePaintFingerprint equals resolvedStateFingerprint for every row and scheme', () => {
    for (const scheme of schemes) {
      for (const state of controlStateNames) {
        expect(resolvePaintFingerprint(effectiveStatePaint(state), scheme)).toBe(
          resolvedStateFingerprint(state, scheme),
        );
      }
    }
  });

  test('the equivalence is not vacuous: the two functions disagree about a changed paint', () => {
    // If `composeStatePaint` ignored its argument the assertions above would pass anyway. This is
    // the anti-vacuity half: a spec that differs from the table must compose to something else.
    const changed: ControlStateSpec = { ...controlStateSpecs.hover, background: 'accentSurface' };
    expect(composeStatePaint(changed, controlStateSpecs.rest)).not.toEqual(
      effectiveStatePaint('hover'),
    );
  });
});

// ── the three derived tables ─────────────────────────────────────────────────

const tables: readonly {
  name: string;
  table: Readonly<Record<string, DerivedStateSpec>>;
  names: readonly string[];
  cascade: readonly string[];
  /** What a `transparent` fill in this table actually sits on. See `resolve` below. */
  substrate: string;
  paint: (state: string) => EffectiveStatePaint;
  fingerprint: (state: string, scheme: ColorScheme) => string;
}[] = [
  {
    name: 'field',
    substrate: 'surface',
    table: fieldStates,
    names: fieldStateNames,
    cascade: fieldStateCascade,
    paint: (state) => fieldPaint(state as (typeof fieldStateNames)[number]),
    fingerprint: (state, scheme) =>
      resolvedFieldFingerprint(state as (typeof fieldStateNames)[number], scheme),
  },
  {
    name: 'box',
    // A checkbox sits in a pane, on the raised surface.
    substrate: 'surface',
    table: boxStates,
    names: boxStateNames,
    cascade: boxStateCascade,
    paint: (state) => boxPaint(state as (typeof boxStateNames)[number]),
    fingerprint: (state, scheme) =>
      resolvedBoxFingerprint(state as (typeof boxStateNames)[number], scheme),
  },
  {
    name: 'option',
    // An option sits inside the popup list, which is drawn at the overlay rung.
    substrate: 'surfaceRaised',
    table: optionStates,
    names: optionStateNames,
    cascade: optionStateCascade,
    paint: (state) => optionPaint(state as (typeof optionStateNames)[number]),
    fingerprint: (state, scheme) =>
      resolvedOptionFingerprint(state as (typeof optionStateNames)[number], scheme),
  },
];

describe.each(tables)('the $name state table', ({ name, table, names, cascade, fingerprint }) => {
  test('every row describes itself, and every selector is a template over the element', () => {
    for (const state of names) {
      const entry = table[state];
      expect(entry, `${state} is missing from the ${name} table`).toBeDefined();
      if (entry === undefined) continue;
      expect(entry.description.trim(), `${state} has no description`).not.toBe('');
      for (const match of entry.matches) {
        expect(match, `${state} has a selector that never mentions the element`).toContain('%s');
      }
    }
  });

  test('a locally declared paint says why the shared table could not supply it', () => {
    let borrowed = 0;
    for (const state of names) {
      const entry = table[state];
      if (entry === undefined) continue;
      if (typeof entry.paint === 'string') {
        borrowed += 1;
        expect(controlStateNames, `${state} names a control state that does not exist`).toContain(
          entry.paint,
        );
        expect(entry.because, `${state} borrows a paint and also explains itself`).toBeUndefined();
      } else {
        expect(
          (entry.because ?? '').trim(),
          `${state} declares its own paint and does not say why`,
        ).not.toBe('');
      }
    }
    // Anti-vacuity: a table that declared *everything* itself would pass the loop above and would
    // have thrown the shared answer away, which is the failure the rule exists to prevent.
    expect(borrowed, `the ${name} table borrows nothing from the shared control table`).toBeGreaterThan(0);
  });

  test('the cascade is exactly the states that paint, once each', () => {
    const painting = names.filter((state) => (table[state]?.matches.length ?? 0) > 0);
    expect([...cascade].sort()).toEqual([...painting].sort());
    expect(new Set(cascade).size).toBe(cascade.length);
  });

  test('no two states resolve alike, in either scheme', () => {
    for (const scheme of schemes) {
      const seen = new Map<string, string>();
      for (const state of names) {
        const print = fingerprint(state, scheme);
        const clash = seen.get(print);
        expect(clash, `${state} and ${String(clash)} are identical in ${scheme}`).toBeUndefined();
        seen.set(print, state);
      }
    }
  });

  test('the text of every state is legible on the fill of that state, in both schemes', () => {
    for (const scheme of schemes) {
      for (const state of names) {
        const entry = table[state];
        if (entry === undefined) continue;
        const paint = tables.find((candidate) => candidate.name === name)?.paint(state);
        if (paint === undefined) continue;
        // A dimmed state is exempt for the reason `disabledOpacity` gives: axe's contrast rule does
        // not apply to a disabled control, and the *explained* state is at full opacity precisely
        // so that it is not exempt.
        if (paint.opacity < 1) continue;
        const measured = ratio(paint.text, paint.background, scheme);
        expect(
          measured,
          `${name} · ${state}: ${paint.text} on ${String(paint.background)} is ${measured.toFixed(2)} : 1 in ${scheme}`,
        ).toBeGreaterThanOrEqual(bodyTextMinimum);
      }
    }
  });
});

// ── the indicator rule, and its proof of failability ─────────────────────────

/**
 * **What says a state is on, without reading its text.**
 *
 * WCAG 2.2 §1.4.11 wants the *visual information required to identify a state* to clear 3 : 1, and
 * the mistake to avoid is assuming that information is always a colour. Five mechanisms can carry
 * a state and they are checked in order, so each state is attributed to the strongest one it has:
 *
 * | Mechanism | What it is |
 * |---|---|
 * | `ring` | an inset ring, measured against the state's own fill |
 * | `border` | a border colour differing from resting, measured against the state's own fill |
 * | `fill` | a fill differing from resting, measured against the **resting** fill |
 * | `weight` | a font weight or border style differing from resting — **not a colour at all**, so it carries a state for a person who cannot tell two greens apart |
 * | `declared` | the model says what else says it, and the browser gate asserts that thing is drawn |
 *
 * Returning the mechanism rather than a boolean is what makes this an assertion of
 * **correspondence**: the whole map is written out below, so a state sliding from `ring` to
 * `declared` is a diff somebody has to write down rather than a threshold that quietly still
 * passes. That is MJXOFF-185's lesson — *a gate that compares a generated thing against the thing
 * it was generated from stays green under a real break* — applied to a contrast rule.
 */
type Mechanism = 'ring' | 'border' | 'fill' | 'weight' | 'declared' | 'none' | 'baseline';

/**
 * Strongest first. A state carried by different mechanisms in the two schemes is recorded as the
 * **weaker** of them, because a control has to work in both.
 */
const mechanismStrength: readonly Mechanism[] = [
  'baseline',
  'ring',
  'border',
  'fill',
  'weight',
  'declared',
  'none',
];

/**
 * ⚠ **What a `transparent` paint actually is.**
 *
 * The first version of this gate returned `Infinity` for any comparison involving `transparent`,
 * which made every such comparison pass — and an option's resting fill *is* transparent, so
 * `selected` and `unavailable` were both being credited with a colour indicator that had never
 * been measured. The gate found it in itself, which is the only reason it is written down here
 * rather than shipped.
 *
 * A transparent fill is the surface behind it, and only the table knows which surface that is: a
 * field paints its own, and an option sits on the card its list is drawn as. So the substrate is
 * data, per table, and `transparent` resolves to it before anything is measured.
 */
function resolve(paint: string | undefined, substrate: string): string | undefined {
  if (paint === undefined) return undefined;
  return paint === 'transparent' ? substrate : paint;
}

function carriedBy(
  entry: DerivedStateSpec,
  state: string,
  paint: EffectiveStatePaint,
  resting: EffectiveStatePaint,
  substrate: string,
  scheme: ColorScheme,
): Mechanism {
  if (state === 'rest') return 'baseline';
  // `focus` paints nothing of its own: the foundations' ring is its indicator, and
  // `tests/browser/foundations.spec.ts` is what measures that ring.
  if (paint.ring) return 'baseline';

  const fill = resolve(paint.background, substrate) ?? substrate;
  const restingFill = resolve(resting.background, substrate) ?? substrate;

  if (paint.insetRing !== undefined) {
    return ratio(paint.insetRing, fill, scheme) >= nonTextMinimum ? 'ring' : 'none';
  }
  if (paint.borderColor !== resting.borderColor && paint.borderColor !== 'transparent') {
    if (ratio(paint.borderColor, fill, scheme) >= nonTextMinimum) return 'border';
  }
  if (fill !== restingFill && ratio(fill, restingFill, scheme) >= nonTextMinimum) return 'fill';
  if (paint.weight !== resting.weight || paint.borderStyle !== resting.borderStyle) return 'weight';
  if ((entry.nonColourCue ?? '').trim() !== '') return 'declared';
  return 'none';
}

function mechanismMap(
  table: Readonly<Record<string, DerivedStateSpec>>,
  names: readonly string[],
  paintOf: (state: string) => EffectiveStatePaint,
  substrate: string,
): Record<string, Mechanism> {
  const resting = paintOf('rest');
  const map: Record<string, Mechanism> = {};
  for (const state of names) {
    const entry = table[state];
    if (entry === undefined) continue;
    let weakest: Mechanism = 'baseline';
    for (const scheme of schemes) {
      const found = carriedBy(entry, state, paintOf(state), resting, substrate, scheme);
      if (mechanismStrength.indexOf(found) > mechanismStrength.indexOf(weakest)) weakest = found;
    }
    map[state] = weakest;
  }
  return map;
}

/** What carries each state. Written out, so a change to one of them is a change here. */
const expectedMechanisms: Readonly<Record<string, Readonly<Record<string, Mechanism>>>> = {
  field: {
    rest: 'baseline',
    hover: 'border',
    editing: 'ring',
    // 2.07 : 1 in light. The warning glyph and the message beneath it are what actually carry it.
    invalid: 'declared',
    unavailable: 'weight',
    disabled: 'declared',
    focus: 'baseline',
  },
  box: {
    rest: 'baseline',
    hover: 'border',
    // The bold-weight difference stands in for the real cue, which is the glyph: a check mark, a
    // dash, or nothing. `boxGlyphs` is asserted separately to draw three different things.
    checked: 'weight',
    mixed: 'weight',
  },
  option: {
    rest: 'baseline',
    active: 'ring',
    selected: 'weight',
    selectedActive: 'ring',
    unavailable: 'weight',
  },
};

describe('the indicator rule', () => {
  test.each(tables)(
    '$name: what carries each state is what the model says carries it',
    ({ name, table, names, paint, substrate }) => {
      expect(mechanismMap(table, names, paint, substrate), name).toEqual(expectedMechanisms[name]);
    },
  );

  test.each(tables)(
    '$name: no state is carried by nothing',
    ({ name, table, names, paint, substrate }) => {
      const map = mechanismMap(table, names, paint, substrate);
      for (const [state, mechanism] of Object.entries(map)) {
        expect(mechanism, `${name} · ${state} is indicated by nothing at all`).not.toBe('none');
      }
      // ⚠ Anti-vacuity. A table every one of whose states was carried by a *declaration* would
      // satisfy the assertion above while measuring no contrast at all — which is the shape of the
      // two green gates MJXOFF-185 found under a real break.
      const measured = Object.values(map).filter(
        (mechanism) => mechanism === 'ring' || mechanism === 'border' || mechanism === 'fill',
      );
      expect(measured.length, `${name} measured no colour indicator at all`).toBeGreaterThan(0);
    },
  );

  test('the rule can fail: a state with a weak colour and nothing else is reported as `none`', () => {
    const resting: ControlStateSpec = {
      description: '',
      background: 'surface',
      borderColor: 'border',
      borderStyle: 'solid',
      text: 'textPrimary',
      weight: 'medium',
      opacity: 1,
      matches: [],
    };
    // `accentBorder` on `surface` is 1.36 : 1 in light — a real pairing, deliberately chosen, so
    // this fixture fails for the reason a real mistake would.
    const weak: DerivedStateSpec = {
      paint: { description: '', borderColor: 'accentBorder', matches: [] },
      because: 'a fixture',
      description: 'a state whose only signal is a colour nobody can see',
      matches: ['%s[data-weak]'],
    };
    const paint = composeStatePaint(paintSpecOf(weak), resting);
    const restingPaint = composeStatePaint(resting, resting);
    expect(carriedBy(weak, 'weak', paint, restingPaint, 'surface', 'light')).toBe('none');

    // …and the same state with a cue declared is carried by the declaration, which is the escape
    // hatch working rather than the gate failing.
    const cued: DerivedStateSpec = { ...weak, nonColourCue: 'a warning glyph' };
    expect(carriedBy(cued, 'weak', paint, restingPaint, 'surface', 'light')).toBe('declared');

    // …and the vacuity the first version of this gate had: a transparent resting fill must not
    // make an unmeasurable comparison count as a measured one.
    const transparentResting: ControlStateSpec = { ...resting, background: 'transparent' };
    const tinted: DerivedStateSpec = {
      paint: { description: '', background: 'accentSurface', matches: [] },
      because: 'a fixture',
      description: 'a fill that differs from a transparent resting fill by 1.14 : 1',
      matches: ['%s[data-tinted]'],
    };
    expect(
      carriedBy(
        tinted,
        'tinted',
        composeStatePaint(paintSpecOf(tinted), transparentResting),
        composeStatePaint(transparentResting, transparentResting),
        'surface',
        'light',
      ),
    ).toBe('none');
  });

  test('every declared cue is a sentence, and the set of them is named', () => {
    const declared: string[] = [];
    for (const { name, table, names } of tables) {
      for (const state of names) {
        const entry = table[state];
        if (entry?.nonColourCue === undefined) continue;
        expect(entry.nonColourCue.trim(), `${name} · ${state} declares an empty cue`).not.toBe('');
        declared.push(`${name}.${state}`);
      }
    }
    // Named rather than counted, so adding a fifth is a decision somebody has to write down here.
    expect(declared.sort()).toEqual([
      'field.disabled',
      'field.invalid',
      'field.unavailable',
      'option.unavailable',
    ]);
  });

  test('the invalid state’s border really is the one that cannot carry it', () => {
    // The model says 2.07 : 1 in light. Asserted, so the paragraph cannot become false without
    // this failing — and asserted as a *ceiling*, because a re-seed that fixed it should make
    // somebody delete the escape hatch rather than leave it standing.
    const measured = ratio('secondaryAccent', 'surface', 'light');
    expect(measured).toBeLessThan(nonTextMinimum);
    expect(fieldStates.invalid.nonColourCue).toBeDefined();
  });

  test('the borrowed `on` paint is carried by its weight and not by its edge', () => {
    // ⚠ A finding worth writing down, and one this child did not go looking for.
    // `controlStateSpecs.on` borders with `accentBorder` on an `accentSurface` fill — 1.20 : 1 in
    // light — and fills 1.14 : 1 against white. Neither is an indicator, and what actually
    // distinguishes a pressed toggle, a checked box and a chosen option is the **bold label** the
    // same row declares. That is legitimate; what would not be legitimate is believing the edge
    // was doing it, which is what a reader of the table would assume.
    const on = effectiveStatePaint('on');
    expect(ratio(on.borderColor, on.background, 'light')).toBeLessThan(nonTextMinimum);
    expect(on.weight).toBe('bold');
    expect(effectiveStatePaint('rest').weight).not.toBe('bold');
  });
});

// ── the field's own decision ─────────────────────────────────────────────────

describe('the field table', () => {
  test('the fill is the same in every state — the whole reason this table exists', () => {
    const resting = fieldPaint('rest').background;
    expect(resting).toBe('surface');
    for (const state of fieldStateNames) {
      expect(fieldPaint(state).background, `${state} changes the field's fill`).toBe(resting);
    }
  });

  test('secondary text is legible on that fill in both schemes, and would not be on hover’s', () => {
    for (const scheme of schemes) {
      expect(ratio('textSecondary', 'surface', scheme)).toBeGreaterThanOrEqual(bodyTextMinimum);
    }
    // The measurement the decision was taken from. If a re-seed ever lifts this over 4.5 the
    // constraint stops being load-bearing, and somebody should find out here rather than by
    // reading the comment.
    expect(ratio('textSecondary', 'borderSubtle', 'light')).toBeLessThan(bodyTextMinimum);
  });

  test('focus paints nothing and is not in the cascade', () => {
    expect(fieldStates.focus.matches).toEqual([]);
    expect(fieldStateCascade).not.toContain('focus');
    expect(fieldPaint('focus').ring).toBe(true);
  });

  test('the two unavailable rows come last, so an invalid unavailable field reads as both', () => {
    expect(fieldStateCascade[fieldStateCascade.length - 1]).toBe('disabled');
    expect(fieldStateCascade[fieldStateCascade.length - 2]).toBe('unavailable');
    expect(fieldStateCascade.indexOf('invalid')).toBeGreaterThan(fieldStateCascade.indexOf('editing'));
  });
});

describe('the checkbox box', () => {
  test('there is no checkedHover, because the row hovers instead', () => {
    expect(boxStateNames).not.toContain('checkedHover');
    expect(boxStateNames).not.toContain('mixedHover');
  });

  test('checked and mixed are the shared table’s rows, by name', () => {
    expect(boxStates.checked.paint).toBe('on');
    expect(boxStates.mixed.paint).toBe('mixed');
    expect(boxPaint('checked')).toEqual(
      composeStatePaint(controlStateSpecs.on, paintSpecOf(boxStates.rest)),
    );
  });

  test('the two positions draw different glyphs, and rest draws none', () => {
    expect(boxGlyphs.rest).toBeUndefined();
    expect(boxGlyphs.hover).toBeUndefined();
    expect(boxGlyphs.checked?.name).toBe('checkmark');
    expect(boxGlyphs.mixed?.name).toBe('subtract');
    expect(boxGlyphs.checked?.name).not.toBe(boxGlyphs.mixed?.name);
  });

  test('both glyphs are in the icon subset at the size the box draws them', () => {
    for (const glyph of [boxGlyphs.checked, boxGlyphs.mixed]) {
      if (glyph === undefined) continue;
      const row = iconRequests.find((request) => request.name === glyph.name);
      expect(row, `${glyph.name} is not in the icon manifest`).toBeDefined();
      expect(row?.sizes, `${glyph.name} is not drawn at ${String(glyph.size)}`).toContain(glyph.size);
    }
  });
});

describe('an option', () => {
  test('there is no focus state, because focus never reaches an option', () => {
    expect(optionStateNames).not.toContain('focus');
    for (const state of optionStateNames) {
      expect(optionPaint(state).ring, `${state} claims the foundations' focus ring`).toBe(false);
    }
  });

  test('the keyboard cursor is a state of its own and carries a ring', () => {
    expect(optionPaint('active').insetRing).toBeDefined();
    expect(optionPaint('selectedActive').insetRing).toBeDefined();
    for (const scheme of schemes) {
      expect(ratio(optionPaint('active').insetRing, optionPaint('active').background, scheme))
        .toBeGreaterThanOrEqual(nonTextMinimum);
    }
  });

  test('the ring the control table would have supplied is the one that fails', () => {
    // `controlStateSpecs.onHover` rings with `accent` on an `accentSurface` fill. Measured here so
    // the model's explanation for not borrowing it cannot quietly become untrue.
    const borrowed = effectiveStatePaint('onHover');
    expect(ratio(borrowed.insetRing, borrowed.background, 'light')).toBeLessThan(nonTextMinimum);
    expect(optionStates.selectedActive.paint).not.toBe('onHover');
  });

  test('the second line is told apart by size and never by colour', () => {
    expect(optionSecondaryTextIsSizeNotColour).toBe(true);
    expect(optionCss).not.toContain('text-secondary');
    for (const state of optionStateNames) {
      // Every state's text is the *same* member, so there is no "less important" colour to reach
      // for in the first place.
      expect(['textPrimary', 'textSecondary']).toContain(optionPaint(state).text);
    }
  });

  test('a live row’s three facts resolve to exactly one state', () => {
    expect(optionStateOf({ selected: false, active: false, unavailable: false })).toBe('rest');
    expect(optionStateOf({ selected: false, active: true, unavailable: false })).toBe('active');
    expect(optionStateOf({ selected: true, active: false, unavailable: false })).toBe('selected');
    expect(optionStateOf({ selected: true, active: true, unavailable: false })).toBe('selectedActive');
    expect(optionStateOf({ selected: true, active: true, unavailable: true })).toBe('unavailable');
  });
});

// ── the stylesheets ──────────────────────────────────────────────────────────

describe('the stylesheets', () => {
  const painting = [
    'background:',
    'border-color:',
    'border-style:',
    'color:',
    'font-weight:',
    'opacity:',
    'box-shadow:',
  ];

  function blocksOf(css: string, selector: string): string[] {
    const found: string[] = [];
    const pattern = new RegExp(`\\${selector}\\s*\\{([^}]*)\\}`, 'g');
    for (const match of css.matchAll(pattern)) found.push(match[1] ?? '');
    return found;
  }

  test('the base box rules paint nothing at all', () => {
    // MJXOFF-183's specificity accident: a `.field { background: … }` at (0,1,0) would out-specify
    // every `:where()` state rule at (0,0,0), leaving a field that renders its resting paint in all
    // seven states while every "the state exists" check passed.
    for (const selector of ['.field', '.option', '.box', '.segment']) {
      const css = selector === '.option' ? optionCss : selector === '.segment' ? segmentedSheet : inputBaseCss;
      for (const block of blocksOf(css, selector)) {
        for (const property of painting) {
          expect(block, `${selector} declares ${property}, which out-specifies the state table`)
            .not.toContain(property);
        }
      }
    }
  });

  test('every state rule is wrapped in :where(), so source order is the whole cascade', () => {
    for (const [label, css] of [
      ['field', fieldStatesCss('.field')],
      ['box', boxStatesCss('.box')],
      ['option', optionStatesCss('.option')],
      ['segment', controlStatesCss('.segment')],
    ] as const) {
      const rules = css.split('\n').filter((line) => line.trim().endsWith('{'));
      expect(rules.length, `${label} emitted no state rules`).toBeGreaterThan(0);
      for (const rule of rules) {
        expect(rule.trim(), `${label} has a state rule outside :where()`).toMatch(/^:where\(/);
      }
    }
  });

  test('the rules come out in cascade order', () => {
    const css = fieldStatesCss('.field');
    let cursor = -1;
    for (const state of fieldStateCascade) {
      const first = fieldStates[state].matches[0]?.replaceAll('%s', '.field') ?? '';
      const at = css.indexOf(first);
      expect(at, `${state} is not in the emitted sheet`).toBeGreaterThan(-1);
      expect(at, `${state} is emitted before the state it must beat`).toBeGreaterThan(cursor);
      cursor = at;
    }
  });

  test('a segmented control adds no paint of its own', () => {
    expect(segmentedSheet).toContain(controlStatesCss('.segment'));
    // The stronger claim: a segment's paint is *identical* to a toggle button's, not similar.
    for (const scheme of schemes) {
      for (const state of controlStateNames) {
        expect(resolvePaintFingerprint(effectiveStatePaint(state), scheme)).toBe(
          resolvedStateFingerprint(state, scheme),
        );
      }
    }
  });

  test('the slider reads its fraction from one property, so the fill and the thumb agree', () => {
    const uses = [...sliderCss.matchAll(/var\(--mjx-slider-fraction[^)]*\)/g)];
    expect(uses.length).toBe(2);
  });
});

// ── the slider's paints, measured ────────────────────────────────────────────

describe('the slider’s paints', () => {
  test('every indicator clears 3 : 1 against what it is drawn on, in both schemes', () => {
    for (const scheme of schemes) {
      for (const [front, back] of sliderIndicatorPairs) {
        const measured = ratio(front, back, scheme);
        expect(measured, `${front} on ${back} is ${measured.toFixed(2)} : 1 in ${scheme}`)
          .toBeGreaterThanOrEqual(nonTextMinimum);
      }
    }
  });

  test('the pairing the model rejected really does fail, so the reason cannot rot', () => {
    // `accent` on `borderSubtle` — the obvious filled-track colour — is 2.81 : 1 in light.
    expect(ratio('accent', sliderPaints.rail, 'light')).toBeLessThan(nonTextMinimum);
    expect(sliderPaints.fill).not.toBe('accent');
  });

  test('the tick labels are legible on the surface they are drawn on', () => {
    for (const scheme of schemes) {
      expect(ratio(sliderPaints.tickLabel, sliderLabelBackground, scheme))
        .toBeGreaterThanOrEqual(bodyTextMinimum);
    }
  });
});

// ── slider arithmetic ────────────────────────────────────────────────────────

describe('slider arithmetic', () => {
  const percent: SliderRange = { min: 0, max: 100, step: 5 };
  const spacing: SliderRange = { min: 1, max: 3, step: 0.05 };
  const offBoundary: SliderRange = { min: 0, max: 10, step: 3 };

  test('decimalPlaces reads a number’s own representation', () => {
    expect(decimalPlaces(1)).toBe(0);
    expect(decimalPlaces(0.05)).toBe(2);
    expect(decimalPlaces(0.001)).toBe(3);
    expect(decimalPlaces(1e-7)).toBe(7);
  });

  test('roundTo does not go through exponential notation', () => {
    expect(roundTo(0.1 + 0.2, 2)).toBe(0.3);
    expect(roundTo(-0.005, 2)).toBe(-0.01);
    // ⚠ 1, not 1.01 — and the assertion is deliberately of the true answer rather than of the
    // intuitive one. The double nearest 1.005 is 1.00499999999999989, so *every* scale-and-round
    // implementation gives 1, and a test that demanded 1.01 would be demanding a bug.
    expect(roundTo(1.005, 2)).toBe(1);
  });

  test('the fraction is right at both ends and in the middle', () => {
    expect(sliderFraction(0, 0, 100)).toBe(0);
    expect(sliderFraction(25, 0, 100)).toBe(0.25);
    expect(sliderFraction(50, 0, 100)).toBe(0.5);
    expect(sliderFraction(100, 0, 100)).toBe(1);
    // A range that does not start at zero is the case an implementation forgets.
    expect(sliderFraction(1, 1, 3)).toBe(0);
    expect(sliderFraction(2, 1, 3)).toBe(0.5);
    expect(sliderFraction(3, 1, 3)).toBe(1);
    // A degenerate range has one position, and it is the start of the track.
    expect(sliderFraction(5, 5, 5)).toBe(0);
  });

  test('a value off a step boundary is snapped to the nearest one', () => {
    expect(snapToStep(37, percent)).toBe(35);
    expect(snapToStep(38, percent)).toBe(40);
    expect(snapToStep(1.17, spacing)).toBe(1.15);
    expect(snapToStep(1.18, spacing)).toBe(1.2);
    // …and the float tail is gone, which is the whole reason `roundTo` exists.
    expect(String(snapToStep(1.18, spacing))).toBe('1.2');
  });

  test('the maximum is reachable exactly, even when it is not on a boundary', () => {
    // 0, 3, 6, 9 are the boundaries; 10 is the top. A snap that clamped first and rounded second
    // would put End at 9 on a slider whose label says the top is 10.
    expect(snapToStep(10, offBoundary)).toBe(10);
    // …and it is reachable because it is a *stop*, not because everything lands on it: 9.4 is
    // nearer the boundary at 9 and 9.6 is nearer the top. Without both halves, a function that
    // snapped every value to the maximum would pass this test.
    expect(snapToStep(9.4, offBoundary)).toBe(9);
    expect(snapToStep(9.6, offBoundary)).toBe(10);
    expect(snapToStep(4, offBoundary)).toBe(3);
    expect(applySliderAction('maximum', 0, offBoundary, 3)).toBe(10);
    expect(applySliderAction('minimum', 10, offBoundary, 3)).toBe(0);
    // And a step past the top is clamped to the top rather than refused.
    expect(applySliderAction('increment', 9, offBoundary, 3)).toBe(10);
  });

  test('a keyboard step moves by exactly one step, and stops at the ends', () => {
    expect(applySliderAction('increment', 50, percent, 10)).toBe(55);
    expect(applySliderAction('decrement', 50, percent, 10)).toBe(45);
    expect(applySliderAction('increment', 100, percent, 10)).toBe(100);
    expect(applySliderAction('decrement', 0, percent, 10)).toBe(0);
    expect(applySliderAction('increment', 1.15, spacing, 0.2)).toBe(1.2);
    expect(applySliderAction('decrement', 1.15, spacing, 0.2)).toBe(1.1);
  });

  test('the page step is a tenth of the range, on a boundary, never smaller than one step', () => {
    expect(defaultPageStep(percent)).toBe(10);
    expect(defaultPageStep(spacing)).toBe(0.2);
    // A step coarser than a tenth of the range: the page key must still move.
    expect(defaultPageStep(offBoundary)).toBe(3);
    expect(defaultPageStep({ min: 0, max: 4, step: 1 })).toBe(1);
  });

  test('a click on the rail lands on a step boundary', () => {
    expect(valueAtFraction(0, percent)).toBe(0);
    expect(valueAtFraction(1, percent)).toBe(100);
    expect(valueAtFraction(0.37, percent)).toBe(35);
    expect(valueAtFraction(-1, percent)).toBe(0);
    expect(valueAtFraction(2, percent)).toBe(100);
  });

  test('the inline arrows mirror under RTL and the block arrows never do', () => {
    expect(sliderKeyAction('ArrowRight', 'ltr', 'horizontal')).toBe('increment');
    expect(sliderKeyAction('ArrowRight', 'rtl', 'horizontal')).toBe('decrement');
    expect(sliderKeyAction('ArrowLeft', 'rtl', 'horizontal')).toBe('increment');
    expect(sliderKeyAction('ArrowUp', 'rtl', 'horizontal')).toBe('increment');
    expect(sliderKeyAction('ArrowDown', 'rtl', 'horizontal')).toBe('decrement');
    // A vertical slider does not mirror at all — the case that gets forgotten.
    expect(sliderKeyAction('ArrowRight', 'rtl', 'vertical')).toBe('increment');
    expect(sliderKeyAction('ArrowLeft', 'rtl', 'vertical')).toBe('decrement');
    expect(sliderKeyAction('Home', 'rtl', 'horizontal')).toBe('minimum');
    expect(sliderKeyAction('End', 'rtl', 'horizontal')).toBe('maximum');
    expect(sliderKeyAction('a', 'ltr', 'horizontal')).toBeUndefined();
  });
});

// ── the measure grammar ──────────────────────────────────────────────────────

describe('the measure grammar', () => {
  const points = (text: string, unit: MeasureUnit = 'pt'): number => {
    const parse = parseMeasure(text, unit);
    expect(parse.ok, `${text} did not parse`).toBe(true);
    return parse.ok ? parse.measure.points : Number.NaN;
  };

  test('the four constants that generate every factor', () => {
    expect(measureUnits.pt.points).toBe(1);
    expect(measureUnits.in.points).toBe(72);
    expect(measureUnits.pc.points).toBe(12);
    expect(measureUnits.px.points).toBe(0.75);
    // The metric pair is asserted through the definition rather than through a decimal typed out
    // to seventeen places: an inch is 2.54 centimetres and 25.4 millimetres, exactly, so a
    // measure of that many of them must be exactly one inch of points.
    expect(pointsFrom(2.54, 'cm')).toBeCloseTo(72, 10);
    expect(pointsFrom(25.4, 'mm')).toBeCloseTo(72, 10);
    expect(measureUnits.cm.points).toBeCloseTo(28.3464566929, 9);
    expect(measureUnits.mm.points).toBeCloseTo(2.8346456693, 9);
  });

  test('the numbers, as numbers', () => {
    expect(points('12 pt')).toBe(12);
    expect(points('12pt')).toBe(12);
    expect(points('1"')).toBe(72);
    expect(points('1 in')).toBe(72);
    expect(points('1 inch')).toBe(72);
    expect(points('1 pc')).toBe(12);
    expect(points('16 px')).toBe(12);
    expect(points('2.5 cm')).toBeCloseTo(70.86614173228347, 10);
    expect(points('10 mm')).toBeCloseTo(28.346456692913385, 10);
    expect(points('-3.5 mm')).toBeCloseTo(-9.921259842519685, 10);
    expect(points('0')).toBe(0);
  });

  test('the unit may be implied, and the field’s own is what is implied', () => {
    expect(points('12')).toBe(12);
    expect(points('12', 'cm')).toBeCloseTo(340.15748031496065, 10);
    const parse = parseMeasure('12', 'cm');
    expect(parse.ok && parse.measure.unitWasImplied).toBe(true);
    const explicit = parseMeasure('12 pt', 'cm');
    expect(explicit.ok && explicit.measure.unitWasImplied).toBe(false);
    expect(explicit.ok && explicit.measure.unit).toBe('pt');
  });

  test('both separators are accepted, in either mode, because neither can mean anything else', () => {
    expect(points('1,5 cm')).toBeCloseTo(42.51968503937008, 10);
    expect(points('1.5 cm')).toBeCloseTo(42.51968503937008, 10);
    expect(points('1,5 cm')).toBe(points('1.5 cm'));
    expect(points('.5 in')).toBe(36);
    expect(points(',5 in')).toBe(36);
  });

  test('a spelling is case-insensitive and a curly quote is an inch', () => {
    expect(unitFromSpelling('CM')).toBe('cm');
    expect(unitFromSpelling('In')).toBe('in');
    expect(unitFromSpelling('"')).toBe('in');
    expect(unitFromSpelling('”')).toBe('in');
    expect(unitFromSpelling('″')).toBe('in');
    expect(unitFromSpelling('furlongs')).toBeUndefined();
    expect(unitFromSpelling('')).toBeUndefined();
  });

  test('what it refuses, and which of the three ways it refuses it', () => {
    const refuse = (text: string): { failure: string; offending: string } => {
      const parse = parseMeasure(text);
      expect(parse.ok, `${text} was accepted`).toBe(false);
      return parse.ok ? { failure: '', offending: '' } : parse.error;
    };
    expect(refuse('').failure).toBe('empty');
    expect(refuse('   ').failure).toBe('empty');
    expect(refuse('banana').failure).toBe('notANumber');
    expect(refuse('pt').failure).toBe('notANumber');
    expect(refuse('--5 pt').failure).toBe('notANumber');
    expect(refuse('12 furlongs').failure).toBe('unknownUnit');
    expect(refuse('12 furlongs').offending).toBe('furlongs');
    // `1.2.3` parses `1.2` and is then left with `.3`, which is not a unit — so it is refused with
    // the fragment that defeated it rather than silently read as 1.2.
    expect(refuse('1.2.3').failure).toBe('unknownUnit');
    expect(refuse('1.2.3').offending).toBe('.3');
  });

  test('every failure has a message, and each names what defeated it', () => {
    for (const failure of ['empty', 'notANumber', 'unknownUnit'] as const) {
      const message = measureFailureMessages[failure];
      expect(message.trim()).not.toBe('');
    }
    expect(measureFailureMessages.notANumber).toContain('{offending}');
    expect(measureFailureMessages.unknownUnit).toContain('{offending}');
    expect(measureFailureMessages.unknownUnit).toContain('{units}');
  });

  test('formatting, as strings, with the separator the field declares', () => {
    expect(formatMeasure(12, 'pt')).toBe('12 pt');
    expect(formatMeasure(72, 'in')).toBe('1 in');
    expect(formatMeasure(70.86614173228347, 'cm')).toBe('2.5 cm');
    expect(formatMeasure(70.86614173228347, 'cm', 'comma')).toBe('2,5 cm');
    expect(formatMeasure(12, 'px')).toBe('16 px');
    expect(formatMeasure(12, 'pc')).toBe('1 pc');
    expect(formatMeasure(28.346456692913385, 'mm')).toBe('10 mm');
    expect(formatMeasure(-0.001, 'pt')).toBe('0 pt');
    expect(decimalCharacter('comma')).toBe(',');
    expect(decimalCharacter('dot')).toBe('.');
  });

  test('parse and format round-trip for every unit’s canonical spelling', () => {
    for (const unit of measureUnitNames) {
      const written = formatMeasure(pointsFrom(2.5, unit), unit);
      expect(written, `${unit} did not round-trip`).toBe(`2.5 ${unit}`);
      expect(points(written)).toBeCloseTo(pointsFrom(2.5, unit), 9);
    }
  });

  test('switching the display unit and back is an identity, not a rounding', () => {
    const original = 12;
    for (const unit of measureUnitNames) {
      expect(pointsFrom(pointsIn(original, unit), unit)).toBeCloseTo(original, 10);
    }
  });

  test('roundToPrecision is symmetric about zero', () => {
    expect(roundToPrecision(2.345, 2)).toBe(2.35);
    expect(roundToPrecision(-2.345, 2)).toBe(-2.35);
    expect(roundToPrecision(-0.005, 2)).toBe(-0.01);
  });

  test('clamping, and the predicate that reports it', () => {
    const range = { minimumPoints: 0, maximumPoints: 72 };
    expect(clampMeasure(-5, range)).toBe(0);
    expect(clampMeasure(100, range)).toBe(72);
    expect(clampMeasure(36, range)).toBe(36);
    expect(withinRange(36, range)).toBe(true);
    expect(withinRange(100, range)).toBe(false);
    expect(clampMeasure(100, {})).toBe(100);
  });

  test('a step is taken in the displayed unit, not in points', () => {
    // 72 pt is 2.54 cm. One centimetre-sized step of 0.25 lands on 2.79 cm, which is 79.087 pt —
    // not 73 pt, which is what stepping in points would have given.
    const stepped = stepMeasure(72, 'cm', 0.25, 1);
    expect(pointsIn(stepped, 'cm')).toBeCloseTo(2.79, 10);
    expect(formatMeasure(stepped, 'cm')).toBe('2.79 cm');
    expect(stepMeasure(12, 'pt', 1, 1)).toBe(13);
    expect(stepMeasure(12, 'pt', 1, -1)).toBe(11);
    expect(stepMeasure(0, 'pt', 1, -1, { minimumPoints: 0 })).toBe(0);
  });
});

// ── the list ─────────────────────────────────────────────────────────────────

const options: readonly OptionDescriptor[] = [
  { value: 'cambria', label: 'Cambria' },
  { value: 'candara', label: 'Candara' },
  { value: 'consolas', label: 'Consolas' },
  { value: 'courier-new', label: 'Courier New' },
  { value: 'georgia', label: 'Georgia' },
];

describe('the list model', () => {
  test('a filter keeps the original order, and an empty query keeps everything', () => {
    expect(filterOptions(options, '', 'contains')).toHaveLength(options.length);
    expect(filterOptions(options, 'ca', 'startsWith').map((option) => option.value)).toEqual([
      'cambria',
      'candara',
    ]);
    expect(filterOptions(options, 'ur', 'contains').map((option) => option.value)).toEqual([
      'courier-new',
    ]);
    // The two modes differ, which is the whole reason both exist.
    expect(filterOptions(options, 'ur', 'startsWith')).toEqual([]);
    expect(filterOptions(options, 'zzz', 'contains')).toEqual([]);
    // Case and surrounding space are not a person's mistake.
    expect(filterOptions(options, '  CAM ', 'startsWith').map((option) => option.value)).toEqual([
      'cambria',
    ]);
  });

  test('the displayed text is the value’s label — the invariant the combo box is built on', () => {
    expect(displayTextFor(options, 'cambria', false)).toBe('Cambria');
    expect(displayTextFor(options, '', false)).toBe('');
    // Without `allow-custom` a value the list does not carry has no text at all, which is what
    // makes "the field shows what the control reports" enforceable rather than aspirational.
    expect(displayTextFor(options, 'helvetica', false)).toBe('');
    expect(displayTextFor(options, 'helvetica', true)).toBe('helvetica');
  });

  test('movement wraps at both ends, and the page keys stop instead', () => {
    expect(nextOptionIndex('next', 0, 5)).toBe(1);
    expect(nextOptionIndex('next', 4, 5)).toBe(0);
    expect(nextOptionIndex('previous', 0, 5)).toBe(4);
    expect(nextOptionIndex('first', 3, 5)).toBe(0);
    expect(nextOptionIndex('last', 0, 5)).toBe(4);
    expect(nextOptionIndex('pageNext', 0, 5)).toBe(4);
    expect(nextOptionIndex('pagePrevious', 4, 5)).toBe(0);
    expect(nextOptionIndex('pageNext', 0, 100)).toBe(listboxPageRows);
    expect(nextOptionIndex('next', 0, 0)).toBe(-1);
    // Opening from nothing puts the cursor on the first option rather than on the second.
    expect(nextOptionIndex('next', -1, 5)).toBe(0);
  });

  test('type-ahead searches forward from just after the cursor and cycles', () => {
    const labels = options.map((option) => option.label);
    expect(typeaheadIndex(labels, 'c', -1)).toBe(0);
    expect(typeaheadIndex(labels, 'c', 0)).toBe(1);
    expect(typeaheadIndex(labels, 'c', 2)).toBe(3);
    expect(typeaheadIndex(labels, 'c', 3)).toBe(0);
    expect(typeaheadIndex(labels, 'cou', -1)).toBe(3);
    // A letter nothing starts with leaves the cursor alone rather than sending it to the top.
    expect(typeaheadIndex(labels, 'z', 2)).toBe(-1);
    expect(typeaheadIndex([], 'c', -1)).toBe(-1);
  });

  test('the key map, cell by cell — and the row the two controls invert', () => {
    const dropdown = { open: false, editable: false };
    const dropdownOpen = { open: true, editable: false };
    const combo = { open: false, editable: true };
    const comboOpen = { open: true, editable: true };

    expect(listboxKeyAction('ArrowDown', dropdown)).toBe('open');
    expect(listboxKeyAction('ArrowDown', dropdownOpen)).toBe('next');
    expect(listboxKeyAction('ArrowUp', combo)).toBe('open');
    expect(listboxKeyAction('ArrowUp', comboOpen)).toBe('previous');

    // ⚠ The row that matters. In a select-only dropdown Home and End move through the list; in a
    // text box they are caret keys and the list must not steal them.
    expect(listboxKeyAction('Home', dropdownOpen)).toBe('first');
    expect(listboxKeyAction('End', dropdownOpen)).toBe('last');
    expect(listboxKeyAction('Home', comboOpen)).toBeUndefined();
    expect(listboxKeyAction('End', comboOpen)).toBeUndefined();

    expect(listboxKeyAction('Enter', dropdownOpen)).toBe('commit');
    expect(listboxKeyAction('Enter', comboOpen)).toBe('commit');
    expect(listboxKeyAction('Escape', dropdownOpen)).toBe('close');
    expect(listboxKeyAction('Escape', dropdown)).toBe('revert');
    expect(listboxKeyAction('Tab', dropdownOpen)).toBe('leave');
    expect(listboxKeyAction('Tab', comboOpen)).toBe('leave');

    // Space opens and commits a dropdown, and is a space in a text box.
    expect(listboxKeyAction(' ', dropdown)).toBe('open');
    expect(listboxKeyAction(' ', dropdownOpen)).toBe('commit');
    expect(listboxKeyAction(' ', comboOpen)).toBeUndefined();

    // A printable character is a type-ahead in a dropdown and belongs to the text box otherwise.
    expect(listboxKeyAction('c', dropdown)).toBe('typeahead');
    expect(listboxKeyAction('c', comboOpen)).toBeUndefined();
    expect(listboxKeyAction('F5', dropdownOpen)).toBeUndefined();
    expect(listboxKeyAction('Shift', dropdownOpen)).toBeUndefined();
  });

  test('the page keys only mean something while the list is open', () => {
    expect(listboxKeyAction('PageDown', { open: false, editable: false })).toBeUndefined();
    expect(listboxKeyAction('PageDown', { open: true, editable: false })).toBe('pageNext');
  });
});

describe('the list borrows the gallery’s virtualisation', () => {
  test('a one-column plan is one row per option', () => {
    const plan = galleryRowPlan(new Array<string>(37).fill(''), 1, false);
    expect(plan).toHaveLength(37);
    expect(plan.every((row) => row.kind === 'cells')).toBe(true);
  });

  test('a sectioned plan adds one row per heading, and the headings are in first-seen order', () => {
    const categories = ['Theme fonts', 'Theme fonts', 'All fonts', 'All fonts', 'All fonts'];
    const plan = galleryRowPlan(categories, 1, true);
    expect(plan.filter((row) => row.kind === 'heading')).toHaveLength(2);
    expect(plan).toHaveLength(7);
    expect(plan[0]).toEqual({ kind: 'heading', category: 'Theme fonts' });
  });

  test('the window builds far fewer rows than the list holds, and never zero', () => {
    const plan = galleryRowPlan(new Array<string>(400).fill(''), 1, false);
    const window_ = galleryWindow(plan.length, 0, 8);
    const built = cellsInWindow(plan, window_);
    // The ceiling, and the anti-vacuity assertion beside it: a window that built nothing would
    // satisfy "fewer than four hundred" perfectly.
    expect(built).toBeLessThan(400);
    expect(built).toBeGreaterThan(0);
    expect(built).toBe(9);
    // …and both ends clamp.
    expect(galleryWindow(plan.length, 0, 8).firstRow).toBe(0);
    expect(galleryWindow(plan.length, 399, 8).lastRow).toBe(400);
  });
});

// ── the focus doctrine ───────────────────────────────────────────────────────

describe('the focus doctrine', () => {
  test('every control declares one of the three patterns, and holds one tab stop', () => {
    expect(tabStopsPerControl).toBe(1);
    for (const tag of Object.keys(inputTags) as (keyof typeof inputTags)[]) {
      const pattern = inputFocusPattern[tag];
      expect(Object.keys(inputFocusPatterns), `${tag} declares no focus pattern`).toContain(pattern);
      expect(inputFocusPatterns[pattern].trim()).not.toBe('');
    }
  });

  test('the two list controls keep focus on the field and the segmented control does not', () => {
    expect(inputFocusPattern.dropdown).toBe('activeDescendant');
    expect(inputFocusPattern.comboBox).toBe('activeDescendant');
    expect(inputFocusPattern.segmentedControl).toBe('roving');
  });
});

describe('the density floor', () => {
  test('an option row is at least the accessible hit-target minimum', () => {
    // The value is written into the sheet as an interpolation, so the assertion is that the sheet
    // reads the floor at all — the browser gate measures the resulting box.
    expect(optionCss).toContain('--mjx-option-block-size');
    expect(accessibleHitTargetMinimum).toBe(24);
  });
});
