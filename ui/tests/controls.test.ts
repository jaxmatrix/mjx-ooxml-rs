import { describe, expect, test } from 'vitest';

import { tokens, type ColorScheme } from '../tokens/tokens.ts';
import {
  archetypeStoryTitle,
  archetypeTags,
  componentStateMatrix,
  controlArchetypes,
  controlBaseCss,
  controlSizeNames,
  controlSizes,
  controlStateCascade,
  controlStateNames,
  controlStateSpecs,
  controlStatesCss,
  effectiveStatePaint,
  forcibleStates,
  nextPressed,
  pressedValues,
  derivedMenuLabel,
  resolvedStateFingerprint,
  type ControlState,
} from '../src/controls/control-states.ts';

/**
 * The state table, checked before a browser is involved.
 *
 * `tests/browser/controls.spec.ts` measures what a browser computed, which is the claim that
 * matters. This tier proves the two things a browser cannot say anything useful about:
 *
 * * **the model is internally distinct in both schemes** — and therefore that a palette re-seed
 *   which collapses two states is caught here, named, in a Node test that runs in a second,
 *   rather than in a Playwright run somebody has to interpret; and
 * * **the stylesheet cannot re-acquire the specificity accident it was written to avoid.** Every
 *   state rule is (0,0,0) and the base rule declares none of the properties the state table owns.
 *   Both are asserted below, because MJXOFF-181 spent a child discovering that a rule which
 *   *appears* ordered can be out-specified, and the discovery only happened by breaking two things
 *   at once.
 *
 * ⚠ No colour, size or duration is written in this file. Every expectation is derived from the
 * generated tokens or from the state table itself.
 */

const schemes: readonly ColorScheme[] = ['light', 'dark'];

describe('the state table', () => {
  test('every state is specified, and every selector is a template over the control', () => {
    for (const state of controlStateNames) {
      const spec = controlStateSpecs[state];
      expect(spec.description.trim(), `${state} has no description`).not.toBe('');
      for (const match of spec.matches) {
        expect(match, `${state} has a selector that never mentions the control`).toContain('%s');
      }
    }
  });

  test('the cascade is exactly the states that paint, in a stable order, once each', () => {
    const painting = controlStateNames.filter(
      (state) => controlStateSpecs[state].matches.length > 0,
    );
    expect([...controlStateCascade].sort()).toEqual([...painting].sort());
    expect(new Set(controlStateCascade).size).toBe(controlStateCascade.length);

    // `focus` is the one state with no rule of its own: the foundations own the ring, which is
    // exactly what MJXOFF-181 exists to have prevented a button from re-inventing.
    expect(controlStateCascade).not.toContain('focus');
    expect(controlStateSpecs.focus.ring).toBe(true);
    expect(controlStateSpecs.focus.matches).toEqual([]);

    // Unavailability wins, so an unavailable pressed toggle looks pressed *and* unavailable.
    const last = controlStateCascade.slice(-2);
    expect(last).toEqual(['unavailable', 'disabled']);
  });

  test('a forcible state is presentational, and only hover and active are forcible', () => {
    expect([...forcibleStates]).toEqual(['hover', 'active']);
    for (const state of forcibleStates) {
      const spec = controlStateSpecs[state];
      // A forced state must be reachable *both* ways from one declaration block, or the catalogue
      // is auditing a copy of the component rather than the component.
      expect(spec.matches.some((match) => match.includes('[data-state='))).toBe(true);
      expect(spec.matches.some((match) => match.includes(`:${state}`))).toBe(true);
      expect(spec.matches.length).toBe(2);
    }
  });
});

describe('pairwise distinctness, as MJXOFF-182 asks for it', () => {
  for (const archetype of controlArchetypes) {
    for (const scheme of schemes) {
      test(`${archetype}: no two states of its matrix paint alike in ${scheme}`, () => {
        const seen = new Map<string, ControlState>();
        for (const state of componentStateMatrix[archetype]) {
          const fingerprint = resolvedStateFingerprint(state, scheme);
          const previous = seen.get(fingerprint);
          expect(
            previous,
            `${archetype} renders '${state}' and '${String(previous)}' identically in ${scheme}:\n` +
              `  ${fingerprint}\n` +
              'Two states that render alike pass every "the state exists" check and are invisible ' +
              'to an auditor. If the palette was just re-seeded, this is the pair it collapsed.',
          ).toBeUndefined();
          seen.set(fingerprint, state);
        }
        expect(seen.size).toBe(componentStateMatrix[archetype].length);
      });
    }
  }

  test('pressed is not a shade of hover — they do not even share a colour family', () => {
    // The classic failure, named as its own test so a regression reports the right sentence.
    for (const scheme of schemes) {
      const hover = effectiveStatePaint('hover');
      const on = effectiveStatePaint('on');
      expect(hover.background).not.toBe(on.background);
      expect(
        tokens.theme[scheme][hover.background as 'borderSubtle'],
        `hover and pressed resolve to the same fill in ${scheme}`,
      ).not.toBe(tokens.theme[scheme][on.background as 'accentSurface']);
      // …and pressed says so in a second way, so the distinction survives a monochrome display.
      expect(on.weight).toBe('bold');
      expect(hover.weight).toBe('medium');
    }
  });

  test('mixed is not a paler pressed — it is the other half of the palette', () => {
    const on = effectiveStatePaint('on');
    const mixed = effectiveStatePaint('mixed');
    expect(on.background).toBe('accentSurface');
    expect(mixed.background).toBe('secondarySurface');
    for (const scheme of schemes) {
      expect(resolvedStateFingerprint('mixed', scheme)).not.toBe(
        resolvedStateFingerprint('on', scheme),
      );
    }
  });

  test('disabled and unavailable are told apart, because only one of them is reachable', () => {
    const disabled = effectiveStatePaint('disabled');
    const unavailable = effectiveStatePaint('unavailable');
    // Dimming belongs to the state axe exempts from contrast; the reachable one stays legible.
    expect(disabled.opacity).toBeLessThan(1);
    expect(unavailable.opacity).toBe(1);
    // …and the reachable one carries a mark that says there is a reason to go and read.
    expect(unavailable.borderStyle).toBe('dashed');
    expect(disabled.borderStyle).toBe('solid');
  });
});

describe('the stylesheet cannot re-acquire the specificity accident', () => {
  const css = controlStatesCss('.control');

  test('every state rule is wrapped in :where(), so source order is the only thing deciding', () => {
    const selectors = [...css.matchAll(/^([^{\n]+)\{/gm)].map((match) => (match[1] ?? '').trim());
    expect(selectors.length).toBe(controlStateCascade.length);
    for (const selector of selectors) {
      expect(
        selector.startsWith(':where(') && selector.endsWith(')'),
        `'${selector}' is not entirely inside :where(). A pseudo-class outside it scores, and a ` +
          'rule that appears ordered is then out-specified — which is how MJXOFF-181 lost a whole ' +
          'focus treatment to a rule it had not broken.',
      ).toBe(true);
      // Nothing may escape the wrapper and add specificity after it.
      expect(selector.slice(7, -1)).not.toContain(':where(');
    }
  });

  test('the rules are emitted in cascade order', () => {
    const positions = controlStateCascade.map((state) => {
      const first = controlStateSpecs[state].matches[0] ?? '';
      return css.indexOf(first.replace('%s', '.control'));
    });
    for (const position of positions) expect(position).toBeGreaterThanOrEqual(0);
    expect([...positions].sort((left, right) => left - right)).toEqual(positions);
  });

  test('the base rule declares none of the properties the state table owns', () => {
    // `.control { … }` scores (0,1,0) and would out-specify every state rule. This is the assertion
    // that keeps the two responsibilities apart: the base rule owns the box, the table owns the
    // paint, and neither declares one of the other's properties.
    const owned = ['background', 'border-color', 'border-style', 'color', 'font-weight', 'opacity', 'box-shadow'];
    const base = controlBaseCss.slice(
      controlBaseCss.indexOf('.control {'),
      controlBaseCss.indexOf('}', controlBaseCss.indexOf('.control {')),
    );
    for (const property of owned) {
      expect(
        base,
        `.control declares '${property}', which the state table owns. At (0,1,0) it out-specifies ` +
          'every (0,0,0) state rule, so the control would render its resting paint in all ten ' +
          'states while every "the state exists" check passed.',
      ).not.toContain(`${property}:`);
    }
    // …and it does own the box, so the split above is a division rather than an emptying.
    expect(base).toContain('border-radius:');
    expect(base).toContain('padding-inline:');
  });

  test('every colour in the stylesheet is a var(), and no literal reaches it', () => {
    const colours = [...css.matchAll(/(?:background|border-color|color|box-shadow):\s*([^;]+);/g)];
    expect(colours.length).toBeGreaterThan(0);
    for (const [, value = ''] of colours) {
      const trimmed = value.trim();
      if (trimmed === 'transparent' || trimmed === 'none') continue;
      expect(trimmed, `'${trimmed}' is not a token reference`).toContain('var(--theme-');
    }
  });
});

describe('the size variants', () => {
  test('are three distinct shapes, and only one of them wraps', () => {
    const shapes = controlSizeNames.map((size) => {
      const spec = controlSizes[size];
      return `${String(spec.iconSize)}|${spec.orientation}|${String(spec.labelVisible)}|${String(spec.labelLines)}`;
    });
    expect(new Set(shapes).size).toBe(controlSizeNames.length);
    expect(controlSizes.large.labelLines).toBe(2);
    expect(controlSizes.large.orientation).toBe('column');
    // The icon-only shape hides the label. It never *drops* it — that is the component's job and
    // `tests/browser/controls.spec.ts` reads the accessible name back to prove it.
    expect(controlSizes.icon.labelVisible).toBe(false);
  });
});

describe('the toggle’s three positions', () => {
  test('are the three ARIA allows, and one activation always lands on a definite one', () => {
    expect([...pressedValues]).toEqual(['false', 'true', 'mixed']);
    expect(nextPressed('false')).toBe('true');
    expect(nextPressed('true')).toBe('false');
    // GUESS, and marked as one at its definition: a mixed selection becomes bold rather than
    // losing its formatting.
    expect(nextPressed('mixed')).toBe('true');
  });
});

describe('the catalogue contract the gates read', () => {
  test('every archetype has a tag, a title and a matrix', () => {
    for (const archetype of controlArchetypes) {
      expect(archetypeTags[archetype]).toMatch(/^mjx-/);
      expect(archetypeStoryTitle[archetype]).toMatch(/^Controls\//);
      expect(componentStateMatrix[archetype].length).toBeGreaterThan(0);
      for (const state of componentStateMatrix[archetype]) {
        expect(controlStateNames).toContain(state);
      }
    }
    // Only the toggle claims the four pressed states — a matrix that quietly listed them on a
    // plain button would be six cells of documentation for four cells of behaviour.
    for (const state of ['on', 'onHover', 'mixed', 'mixedHover'] as const) {
      const claimants = controlArchetypes.filter((archetype) =>
        (componentStateMatrix[archetype] as readonly ControlState[]).includes(state),
      );
      expect(claimants).toEqual(['toggleButton']);
    }
  });

  test('a split button’s arrow gets a name of its own even when nobody gives it one', () => {
    expect(derivedMenuLabel('Undo')).toBe('More Undo options');
    expect(derivedMenuLabel('Undo')).not.toBe('Undo');
  });
});
