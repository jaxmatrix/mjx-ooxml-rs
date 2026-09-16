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
  GroupSettleBatch,
  placeGroupCommands,
  presentedCommandOrder,
  ribbonCss,
  ribbonGroupCss,
  ribbonStates,
  ribbonStateNames,
  tabStripPickerAtOrBelow,
  tabStripPresentationAt,
  survivorPlacement,
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

  it('declares Font’s and Paragraph’s survivors between other commands, so the order gate can fail', () => {
    // The browser gate asserts the worst case presents its commands in declared order. That is only
    // evidence if a survivors-first or survivors-last row would present them differently — which it
    // would not have before unit 2b, when the specimen listed its survivors first.
    for (const label of ['Font', 'Paragraph']) {
      const group = wordTabHomeGroups.find((entry) => entry.label === label);
      expect(group, `the specimen has no ${label} group`).toBeDefined();
      const named = group?.named ?? [];
      expect(named[0]?.essential, `${label}'s first named command is a survivor`).not.toBe(true);
      const all = group === undefined ? [] : specimenCommands(group);
      expect(all.at(-1)?.essential, `${label}'s last command is a survivor`).not.toBe(true);
      expect(named.some((command) => command.essential === true)).toBe(true);
    }
  });

  it('demotes nothing that opens a popup, by the census’s judgement of what Office draws', () => {
    // Rule 1 of `demotionRules` is judged on the shape *Office* draws, which a specimen of generic
    // buttons cannot express. `dev/ribbons/census.ts` records that judgement for Word's Home tab:
    // Paste, Find and Underline are split buttons there. This specimen follows it, and says so by
    // name, because a specimen that kept Find while the census refused it is exactly how the two
    // drifted before.
    const splitButtonsInWord = ['Paste', 'Find', 'Underline'];
    const essential = wordTabHomeGroups.flatMap((group) =>
      group.named.filter((entry) => entry.essential === true),
    );
    expect(essential.length, 'the specimen declares no survivor at all').toBeGreaterThan(0);
    for (const command of essential) {
      expect(command.icon, `${command.label} is essential and has no icon`).toBeDefined();
      expect(
        splitButtonsInWord,
        `${command.label} is essential in the specimen and is a split button in Word`,
      ).not.toContain(command.label);
    }
  });
});

// ── where a survivor draws ───────────────────────────────────────────────────

describe('where a survivor draws', () => {
  const commands = [
    { name: 'grow', essential: false },
    { name: 'bold', essential: true },
    { name: 'strike', essential: false },
    { name: 'italic', essential: true },
    { name: 'highlight', essential: false },
  ] as const;
  const isEssential = (command: (typeof commands)[number]): boolean => command.essential;
  const names = (list: readonly (typeof commands)[number][]): string[] => list.map((c) => c.name);

  it('keeps every command in the panel, in declared order, whenever the panel is part of the strip', () => {
    for (const presentation of ['full', 'reduced'] as const) {
      const placement = placeGroupCommands(presentation, commands, isEssential);
      expect(placement.survivors, `${presentation} has a survivor row`).toEqual([]);
      expect(names(placement.panel)).toEqual(['grow', 'bold', 'strike', 'italic', 'highlight']);
    }
  });

  it('moves the survivors beside the trigger when collapsed, keeping both halves in declared order', () => {
    const placement = placeGroupCommands('collapsed', commands, isEssential);
    expect(names(placement.survivors)).toEqual(['bold', 'italic']);
    expect(names(placement.panel)).toEqual(['grow', 'strike', 'highlight']);
    expect(names(presentedCommandOrder('collapsed', commands, isEssential))).toEqual([
      'bold',
      'italic',
      'grow',
      'strike',
      'highlight',
    ]);
  });

  it('splits exactly where the presentation table says the panel is a popup', () => {
    for (const presentation of groupPresentationOrder) {
      const placement = placeGroupCommands(presentation, commands, isEssential);
      expect(placement.survivors.length > 0, presentation).toBe(
        groupPresentations[presentation].panelIsPopup,
      );
      // Nothing is ever removed: the two halves together are every command, once.
      expect([...names(placement.survivors), ...names(placement.panel)].sort()).toEqual(
        names(commands).sort(),
      );
    }
  });

  it('draws the survivor row only in the collapsed presentation, and never as display:none', () => {
    const css = groupPresentationCss();
    expect(css).toContain('--mjx-group-essential-display: flex;');
    expect(css).toContain('--mjx-group-essential-display: contents;');
    expect(css).not.toContain('--mjx-group-essential-display: none;');
    expect(declarationsOf(ribbonGroupCss, '.essential')).toContain(
      'display: var(--mjx-group-essential-display, contents);',
    );
  });

  it('states the decision, its mechanism and what it rejected', () => {
    for (const field of [
      survivorPlacement.rule,
      survivorPlacement.because,
      survivorPlacement.mechanism,
      survivorPlacement.checkedBy,
    ]) {
      expect(field.trim()).not.toBe('');
    }
    expect(survivorPlacement.rejected.length).toBeGreaterThanOrEqual(3);
    expect(survivorPlacement.rejected.some((entry) => entry.alternative.includes('d01cf93'))).toBe(
      true,
    );
  });
});

// ── settling a ribbon-wide change ────────────────────────────────────────────

/**
 * `GroupSettleBatch`, which is the component's *only* route from *"CSS's answer may have changed"*
 * to a re-slot — MJXOFF-342.
 *
 * The property under test is not that settling works; `tests/browser/ribbon.spec.ts` reads the real
 * slots at three widths for that. It is the **order** the work happens in. A style read that
 * follows a slot write is a forced recalculation, so a ribbon-wide `simplified` toggle over N
 * groups costs N of them if each group settles inside its own `attributeChangedCallback`, and one
 * if every group is read before any is written. That is a property of a sequence, so it is asserted
 * over a sequence — in Node, with the flush driven by hand, for the reason `ToastQueue` states.
 */
describe('settling a ribbon-wide change', () => {
  /** A group as the batch sees one: something to read, something to write, nothing else. */
  interface FakeGroup {
    readonly label: string;
    presentation: GroupPresentation;
    connected: boolean;
    settledTo: GroupPresentation | undefined;
    survivors: readonly string[] | undefined;
  }

  const commands = [
    { name: 'grow', essential: false },
    { name: 'bold', essential: true },
    { name: 'strike', essential: false },
  ] as const;

  interface Step {
    readonly kind: 'read' | 'write';
    readonly label: string;
  }

  function fakeGroup(label: string, presentation: GroupPresentation = 'full'): FakeGroup {
    return { label, presentation, connected: true, settledTo: undefined, survivors: undefined };
  }

  /** The batch with its three operations logged and its flush held, so a test can run time. */
  function harness(options: { readonly onWrite?: (group: FakeGroup) => void } = {}) {
    const steps: Step[] = [];
    const deferred: (() => void)[] = [];
    const batch = new GroupSettleBatch<FakeGroup>({
      settles: (group) => group.connected,
      read: (group) => {
        steps.push({ kind: 'read', label: group.label });
        return group.presentation;
      },
      write: (group, presentation) => {
        steps.push({ kind: 'write', label: group.label });
        group.settledTo = presentation;
        group.survivors = placeGroupCommands(
          presentation,
          commands,
          (command) => command.essential,
        ).survivors.map((command) => command.name);
        options.onWrite?.(group);
      },
      defer: (flush) => {
        deferred.push(flush);
      },
    });
    /** Run every deferred flush, including any a write arranged. */
    const settle = (): void => {
      let guard = 0;
      while (deferred.length > 0) {
        guard += 1;
        expect(guard, 'the batch is deferring flushes without end').toBeLessThan(10);
        deferred.shift()?.();
      }
    };
    return { batch, steps, deferred, settle };
  }

  /** A read that happens after any write has happened: one forced style recalculation. */
  function forcedRecalculations(steps: readonly Step[]): number {
    let written = false;
    let forced = 0;
    for (const step of steps) {
      if (step.kind === 'write') written = true;
      else if (written) forced += 1;
    }
    return forced;
  }

  it('reads every group before it writes any of them, however many there are', () => {
    // The shape a ribbon-wide `simplified` toggle makes: <mjx-ribbon> writes the attribute on every
    // group it owns in one loop, so the batch receives N requests and no flush until the loop ends.
    for (const count of [1, 4, 40]) {
      const groups = Array.from({ length: count }, (_, index) => fakeGroup(`g${String(index)}`));
      const { batch, steps, deferred, settle } = harness();
      for (const group of groups) batch.request(group);

      expect(steps, `${String(count)} groups settled inside the loop`).toEqual([]);
      expect(deferred.length, 'N requests arranged more than one flush').toBe(1);

      settle();
      expect(steps.map((step) => step.kind)).toEqual([
        ...Array.from({ length: count }, () => 'read'),
        ...Array.from({ length: count }, () => 'write'),
      ]);
    }
  });

  it('costs the same number of forced style recalculations at 1 group and at 40', () => {
    // The bound the ticket asks for, stated as a number rather than as a shape. Settling inside
    // `attributeChangedCallback` — what this replaced — makes this count `groups - 1`: 0, 3, 39.
    const counts = [1, 4, 40];
    const forced = counts.map((count) => {
      const { batch, steps, settle } = harness();
      for (let index = 0; index < count; index += 1) batch.request(fakeGroup(`g${String(index)}`));
      settle();
      return forcedRecalculations(steps);
    });
    expect(forced).toEqual([0, 0, 0]);
    expect(new Set(forced).size, 'the cost of a toggle depends on how many groups there are').toBe(
      1,
    );
  });

  it('reads and writes a group queued twice in one tick exactly once', () => {
    const group = fakeGroup('font');
    const { batch, steps, settle } = harness();
    for (let index = 0; index < 5; index += 1) batch.request(group);
    expect(batch.queued).toBe(1);
    settle();
    expect(steps).toEqual([
      { kind: 'read', label: 'font' },
      { kind: 'write', label: 'font' },
    ]);
  });

  it('settles each group to its own presentation, and places its commands for it', () => {
    // Reading first must not cost correctness: each group is written with the answer *it* read.
    const groups = [
      fakeGroup('font', 'full'),
      fakeGroup('paragraph', 'reduced'),
      fakeGroup('styles', 'collapsed'),
    ];
    const { batch, settle } = harness();
    for (const group of groups) batch.request(group);
    settle();

    expect(groups.map((group) => group.settledTo)).toEqual(['full', 'reduced', 'collapsed']);
    expect(groups.map((group) => group.survivors)).toEqual([[], [], ['bold']]);
  });

  it('neither reads nor writes a group that disconnected before the flush', () => {
    const staying = fakeGroup('font');
    const leaving = fakeGroup('styles');
    const { batch, steps, settle } = harness();
    batch.request(staying);
    batch.request(leaving);
    leaving.connected = false;
    settle();

    expect(steps.map((step) => step.label)).toEqual(['font', 'font']);
    expect(leaving.settledTo, 'a disconnected group was settled').toBeUndefined();
  });

  it('arranges a new flush for a request made after one drained', () => {
    const group = fakeGroup('font');
    const { batch, steps, deferred, settle } = harness();
    batch.request(group);
    settle();
    expect(deferred.length).toBe(0);

    group.presentation = 'collapsed';
    batch.request(group);
    expect(deferred.length, 'a request after a flush arranged nothing, so it would be lost').toBe(1);
    settle();
    expect(steps.length).toBe(4);
    expect(group.settledTo).toBe('collapsed');
  });

  it('keeps a request made during a write for the next flush, rather than dropping it', () => {
    // A write closes a popup, which dispatches an event, which a host may answer by setting an
    // attribute — a request arriving while the batch is walking its own list.
    const font = fakeGroup('font');
    const styles = fakeGroup('styles', 'collapsed');
    let requested = false;
    const harnessed = harness({
      onWrite: (group) => {
        if (group !== font || requested) return;
        requested = true;
        harnessed.batch.request(styles);
      },
    });
    harnessed.batch.request(font);
    harnessed.settle();

    expect(harnessed.steps.map((step) => `${step.kind}:${step.label}`)).toEqual([
      'read:font',
      'write:font',
      'read:styles',
      'write:styles',
    ]);
    expect(styles.settledTo).toBe('collapsed');
  });

  it('is the only route <mjx-ribbon-group> takes from an attribute to a re-slot', () => {
    // The batching above is a property of a class the component has to actually use. Node cannot
    // upgrade a custom element — this tier has no DOM at all — so what is checked here is that the
    // attribute path still hands the group to the batch instead of settling inside the callback.
    const source = readFileSync(
      resolve(import.meta.dirname, '../src/ribbon/ribbon-group.ts'),
      'utf8',
    );
    // The method, not the module note that names it and not the batch's own `write`, both of which
    // mention settling for good reasons.
    const start = source.indexOf('\n  attributeChangedCallback(');
    expect(start, 'attributeChangedCallback was found in no recognisable shape').toBeGreaterThan(-1);
    const body = source.slice(start, source.indexOf('\n  }', start));
    expect(body, 'the attribute path no longer re-renders at all').toContain('render()');
    expect(body).toContain('#settleBatch.request(this)');
    expect(
      body.includes('#settle('),
      'attributeChangedCallback settles inline again, which is one forced style read per group ' +
        'for a ribbon-wide simplified toggle. See GroupSettleBatch in ribbon-model.ts.',
    ).toBe(false);
    // The probe callback is the one path that may drain synchronously: it already runs before paint.
    expect(source).toContain('MjxRibbonGroup.#settleBatch.flush()');
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
