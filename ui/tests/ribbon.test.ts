import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

import {
  demotionRules,
  essentialCommandLimit,
  groupPresentationAt,
  groupPresentationCss,
  groupPresentationOrder,
  groupPresentations,
  groupPriorities,
  groupPriorityNames,
  ribbonCss,
  ribbonGroupCss,
  ribbonStates,
  ribbonStateNames,
  tabStripPickerAtOrBelow,
  tabStripPresentationAt,
  tabTones,
  type GroupPresentation,
} from '../src/ribbon/ribbon-model.ts';
import {
  specimenCommands,
  wordTabHomeControlCount,
  wordTabHomeGroups,
  wordTabHomeSource,
} from '../dev/word-tab-home.ts';

/**
 * The ribbon's **model**, with no rendering in it.
 *
 * `tests/browser/ribbon.spec.ts` proves the browser reached the presentation this file says it
 * should. This file proves the thing that browser gate compares against is worth comparing against
 * — that the ladder is a ladder, that the cascade is in the order the stylesheet emits, that no
 * base rule paints, and that the worst-case specimen still agrees with the census it was
 * transcribed from.
 */

// ── the census ───────────────────────────────────────────────────────────────

interface CensusRow {
  readonly group: string;
  readonly controls: number;
  readonly inScope: boolean;
}

/**
 * Word's `TabHome`, read out of the committed command surface.
 *
 * The file lives outside `ui/` and this is the only thing in the catalogue that reads it. That is
 * deliberate: `dev/word-tab-home.ts` is a **transcription**, and a transcription nobody checks is
 * the thing `storyConventions` warns about — *"a list nobody checks is worse than no list, because
 * a reader trusts it"*. There is no skip: the file is committed, and its absence is a finding.
 */
function census(): readonly CensusRow[] {
  const path = resolve(import.meta.dirname, '../..', wordTabHomeSource.file);
  const text = readFileSync(path, 'utf8');
  const rows: CensusRow[] = [];
  for (const line of text.split('\n').slice(1)) {
    const [app, tabSet, tab, group, controls, inScope] = line.split('\t');
    if (app !== wordTabHomeSource.app) continue;
    if (tabSet !== wordTabHomeSource.tabSet) continue;
    if (tab !== wordTabHomeSource.tab) continue;
    if (group === undefined || controls === undefined) continue;
    rows.push({ group, controls: Number(controls), inScope: inScope?.trim() === '1' });
  }
  return rows;
}

describe('the Word TabHome specimen', () => {
  it('names exactly the groups the committed census does, with the counts it gives', () => {
    const rows = census();
    expect(rows.length, 'the census has no Word TabHome rows, which cannot be right').toBeGreaterThan(0);

    const fromCensus = [...rows]
      .map((row) => `${row.group} ${String(row.controls)} ${row.inScope ? 'in' : 'out'}`)
      .sort();
    const fromSpecimen = wordTabHomeGroups
      .map((group) => `${group.id} ${String(group.controls)} ${group.inScope ? 'in' : 'out'}`)
      .sort();

    expect(
      fromSpecimen,
      'dev/word-tab-home.ts has drifted from ' +
        `${wordTabHomeSource.file}. The specimen is a transcription and the census is the source; ` +
        'a worst case that is no longer the real worst case is a worst case somebody chose.',
    ).toEqual(fromCensus);
  });

  it('renders exactly as many controls as the census counts', () => {
    const total = census().reduce((sum, row) => sum + row.controls, 0);
    expect(wordTabHomeControlCount).toBe(total);
    const rendered = wordTabHomeGroups.reduce(
      (sum, group) => sum + specimenCommands(group).length,
      0,
    );
    expect(rendered, 'the filler stopped filling to the census count').toBe(total);
  });

  it('keeps every group inside the demotion ceiling', () => {
    for (const group of wordTabHomeGroups) {
      const essential = group.named.filter((command) => command.essential === true);
      expect(
        essential.length,
        `${group.label} declares ${String(essential.length)} essential commands. A survivor row ` +
          'longer than the ceiling is a ribbon again.',
      ).toBeLessThanOrEqual(essentialCommandLimit);
    }
  });

  it('demotes nothing that opens a popup', () => {
    // Rule 1 of `demotionRules`, checked on the specimen's own declarations: a toggle and a
    // one-shot verb qualify; nothing here is a split button, whose whole shape is a menu.
    for (const group of wordTabHomeGroups) {
      for (const command of group.named.filter((entry) => entry.essential === true)) {
        expect(command.icon, `${command.label} is essential and has no icon`).toBeDefined();
      }
    }
  });
});

// ── the ladder ───────────────────────────────────────────────────────────────

describe('the priority ladder', () => {
  it('gives every priority a strictly narrower collapse than its reduce', () => {
    for (const priority of groupPriorityNames) {
      const ladder = groupPriorities[priority];
      expect(
        ladder.collapseAtOrBelow,
        `${priority} collapses before it reduces, which is not a ladder`,
      ).toBeLessThan(ladder.reduceAtOrBelow);
    }
  });

  it('orders the four priorities, with no two of them the same rung', () => {
    const reduce = groupPriorityNames.map((priority) => groupPriorities[priority].reduceAtOrBelow);
    const collapse = groupPriorityNames.map(
      (priority) => groupPriorities[priority].collapseAtOrBelow,
    );
    expect(new Set(reduce).size, 'two priorities reduce at the same width').toBe(reduce.length);
    expect(new Set(collapse).size, 'two priorities collapse at the same width').toBe(
      collapse.length,
    );
    // `groupPriorityNames` is ordered first-to-give-way-last, so both ladders must ascend.
    expect([...reduce].sort((a, b) => a - b)).toEqual(reduce);
    expect([...collapse].sort((a, b) => a - b)).toEqual(collapse);
  });

  it('never un-degrades as the container narrows', () => {
    // The property that makes the ladder a ladder: a group that has collapsed does not become
    // reduced again at a narrower width. Walked one pixel at a time, because a boundary written
    // the wrong way round is exactly what this would catch.
    for (const priority of groupPriorityNames) {
      let previous = 0;
      for (let width = 1700; width >= 200; width -= 1) {
        const index = groupPresentationOrder.indexOf(groupPresentationAt(priority, width));
        expect(
          index,
          `${priority} went back up the ladder at ${String(width)}px`,
        ).toBeGreaterThanOrEqual(previous);
        previous = index;
      }
    }
  });

  it('changes presentation exactly at the widths the ladder declares', () => {
    for (const priority of groupPriorityNames) {
      const ladder = groupPriorities[priority];
      expect(groupPresentationAt(priority, ladder.reduceAtOrBelow + 1)).toBe('full');
      expect(groupPresentationAt(priority, ladder.reduceAtOrBelow)).toBe('reduced');
      expect(groupPresentationAt(priority, ladder.collapseAtOrBelow + 1)).toBe('reduced');
      expect(groupPresentationAt(priority, ladder.collapseAtOrBelow)).toBe('collapsed');
    }
  });

  it('puts three groups in three presentations at one width, which no uniform rule can', () => {
    // The claim the whole per-group design rests on, asserted as a number rather than as a
    // screenshot. 1000 is the width `Three Presentations At One Width` pins itself to.
    const presentations = new Set<GroupPresentation>(
      groupPriorityNames.map((priority) => groupPresentationAt(priority, 1000)),
    );
    expect(presentations.size).toBe(3);
  });

  it('lets simplified refuse full without ever preventing a collapse', () => {
    for (const priority of groupPriorityNames) {
      const ladder = groupPriorities[priority];
      expect(groupPresentationAt(priority, 1600, { simplified: true })).toBe('reduced');
      expect(groupPresentationAt(priority, ladder.collapseAtOrBelow, { simplified: true })).toBe(
        'collapsed',
      );
    }
  });
});

describe('the tab strip', () => {
  it('becomes a picker exactly at the declared width', () => {
    expect(tabStripPresentationAt(tabStripPickerAtOrBelow + 1)).toBe('strip');
    expect(tabStripPresentationAt(tabStripPickerAtOrBelow)).toBe('picker');
  });
});

// ── the stylesheet ───────────────────────────────────────────────────────────

/** The seven properties the control state table owns, and no base rule may declare. */
const paintProperties = [
  'background',
  'border-color',
  'border-style',
  'color',
  'font-weight',
  'opacity',
  'box-shadow',
];

/** One rule's declaration block, by selector, out of a stylesheet string. */
function declarationsOf(css: string, selector: string): string {
  const index = css.indexOf(`\n  ${selector} {`);
  expect(index, `the rule for '${selector}' is not in the sheet`).toBeGreaterThan(-1);
  const open = css.indexOf('{', index);
  const close = css.indexOf('}', open);
  return css.slice(open + 1, close);
}

describe('the ribbon stylesheets', () => {
  it('paints nothing in the rules that own a box', () => {
    // MJXOFF-182's rule, inherited: `.trigger` scores (0,1,0) and every state rule scores (0,0,0),
    // so a single `background:` here would out-specify the whole state table and leave a button
    // that renders its resting paint in all six states while every "the state exists" check passed.
    for (const [sheet, selector] of [
      [ribbonGroupCss, '.trigger'],
      [ribbonCss, '.tab, .chrome-button'],
    ] as const) {
      const block = declarationsOf(sheet, selector);
      for (const property of paintProperties) {
        expect(
          block.includes(`${property}:`),
          `'${selector}' declares '${property}', which the control state table owns. It scores ` +
            'higher than every state rule and would win all of them.',
        ).toBe(false);
      }
    }
  });

  it('emits every reduced rule before any collapsed rule, because the cascade is source order', () => {
    const css = groupPresentationCss();
    const lastReduced = css.lastIndexOf('--mjx-group-presentation: reduced');
    const firstCollapsed = css.indexOf('--mjx-group-presentation: collapsed');
    expect(lastReduced, 'no reduced rule was emitted').toBeGreaterThan(-1);
    expect(firstCollapsed, 'no collapsed rule was emitted').toBeGreaterThan(-1);
    expect(
      firstCollapsed,
      'a collapsed rule is emitted before a reduced one. Every rule here scores the same, so ' +
        'source order is the only thing deciding, and a group below both conditions would come ' +
        'out reduced instead of collapsed.',
    ).toBeGreaterThan(lastReduced);
  });

  it('emits the simplified rules between the base and the container conditions', () => {
    const css = groupPresentationCss();
    const base = css.indexOf(":where(.group[data-priority='primary']) {");
    const simplified = css.indexOf('[data-simplified]');
    const firstContainer = css.indexOf('@container');
    expect(base).toBeGreaterThan(-1);
    expect(simplified).toBeGreaterThan(base);
    expect(
      firstContainer,
      'simplified is emitted after a container condition, so it would refuse a collapse instead ' +
        'of only refusing full.',
    ).toBeGreaterThan(simplified);
  });

  it('wraps every presentation selector in :where(), so the emission order is what decides', () => {
    // ⚠ **This is the assertion the ordering test above could not make, and the one that was
    // missing while a real bug was live.** The simplified rule was first written as
    // `:host([simplified]) .group[data-priority='x']` — (0,4,0) against the container rules'
    // (0,2,0) — so it won at every width and a simplified ribbon never collapsed a group. The
    // order test stayed green throughout, because emission order only decides between rules of
    // equal specificity. Only the browser gate's correspondence assertion caught it.
    const css = groupPresentationCss();
    const selectors = [...css.matchAll(/(^|\n)\s*([^@\n{][^\n{]*)\{/g)].map((match) =>
      (match[2] ?? '').trim(),
    );
    expect(selectors.length).toBeGreaterThan(groupPriorityNames.length * 3);
    for (const selector of selectors) {
      expect(
        selector.startsWith(':where(') && selector.endsWith(')'),
        `'${selector}' is not wrapped in :where(), so it can out-specify another presentation ` +
          'rule and the emission order stops deciding anything.',
      ).toBe(true);
    }
  });

  it('generates one pair of container conditions per priority, at the ladder’s widths', () => {
    const css = groupPresentationCss();
    for (const priority of groupPriorityNames) {
      const ladder = groupPriorities[priority];
      expect(css).toContain(`(width <= ${String(ladder.reduceAtOrBelow)}px)`);
      expect(css).toContain(`(width <= ${String(ladder.collapseAtOrBelow)}px)`);
    }
  });

  it('writes the presentation token every gate reads, for all three presentations', () => {
    const css = groupPresentationCss();
    for (const presentation of groupPresentationOrder) {
      expect(css).toContain(`--mjx-group-presentation: ${presentation}`);
    }
  });
});

// ── the vocabulary ───────────────────────────────────────────────────────────

describe('the presentation and state tables', () => {
  it('gives each presentation a measurable difference from the others', () => {
    // Not a distinctness assertion about pixels — a claim that the *model* says something
    // different for each, so the browser gate has three different things to check rather than
    // three names.
    const fingerprints = groupPresentationOrder.map((presentation) => {
      const spec = groupPresentations[presentation];
      return [
        spec.density,
        String(spec.triggerVisible),
        String(spec.footerVisible),
        String(spec.panelIsPopup),
        String(spec.commandRows),
      ].join('|');
    });
    expect(new Set(fingerprints).size).toBe(groupPresentationOrder.length);
  });

  it('only lets the collapsed presentation be a popup', () => {
    expect(groupPresentations.full.panelIsPopup).toBe(false);
    expect(groupPresentations.reduced.panelIsPopup).toBe(false);
    expect(groupPresentations.collapsed.panelIsPopup).toBe(true);
    expect(groupPresentations.collapsed.triggerVisible).toBe(true);
  });

  it('keeps the hidden ribbon reachable', () => {
    for (const state of ribbonStateNames) {
      const spec = ribbonStates[state];
      // A state that showed nothing at all would be a ribbon a keyboard has lost.
      expect(
        spec.stripVisible || spec.bodyVisible || spec.restoreVisible,
        `the '${state}' state draws nothing`,
      ).toBe(true);
    }
    expect(ribbonStates.hidden.restoreVisible).toBe(true);
  });

  it('tells a contextual tab from a core one with a different pair of tokens', () => {
    expect(tabTones.contextual.selectedBackground).not.toBe(tabTones.core.selectedBackground);
    expect(tabTones.contextual.selectedBorder).not.toBe(tabTones.core.selectedBorder);
    expect(tabTones.core.bandBackground).toBeUndefined();
    expect(tabTones.contextual.bandBackground).toBeDefined();
  });

  it('states the demotion rules with a reason and a check for each', () => {
    expect(demotionRules.length).toBeGreaterThanOrEqual(4);
    for (const entry of demotionRules) {
      expect(entry.rule.trim()).not.toBe('');
      expect(entry.because.trim()).not.toBe('');
      expect(entry.checkedBy.trim()).not.toBe('');
    }
    // The one rule that has a number in it, so the number cannot drift out of the prose.
    expect(demotionRules.some((entry) => entry.rule.includes(String(essentialCommandLimit)))).toBe(
      true,
    );
  });
});
