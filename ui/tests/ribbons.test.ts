import { readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

import {
  commandSurfaceId,
  essentialCommands,
  everyRibbonCommand,
  everyRibbonTab,
  legacyChartTabReason,
  ribbonApplicationNames,
  ribbonCensus,
  ribbonCensusSource,
  ribbonContextualSets,
  ribbonSurfaceHostNames,
  strongestPriority,
  tabAppearanceNames,
  unbuiltContextualSetReason,
  unbuiltContextualSets,
  type RibbonApplication,
  type RibbonCommand,
  type RibbonContextualSetEntry,
  type RibbonGroupEntry,
  type RibbonSurfaceHost,
  type RibbonTabEntry,
  type TabAppearance,
  type UnbuiltContextualSet,
} from '../dev/ribbons/census.ts';
import { controlSizes, type ControlSize } from '../src/controls/control-states.ts';
import { iconRequests } from '../src/icons/manifest.ts';
import {
  essentialCommandLimit,
  groupPresentationOrder,
  groupPriorityNames,
  presentedCommandOrder,
  type GroupPresentation,
} from '../src/ribbon/ribbon-model.ts';

/**
 * **The ribbon catalogue's model**, with no rendering in it.
 *
 * `tests/ribbon.test.ts` is the *component's* model — the ladder, the cascade, the worst-case
 * specimen. This file is the *content's*: that `dev/ribbons/census.ts` still agrees with the
 * committed command surface, that the priority rubric it states is the rubric it followed, and
 * that every icon a command names is one the subset can actually draw.
 *
 * The last of those is the one worth having. `<mjx-icon>` renders **nothing** for a glyph the
 * subset lacks and reports the miss on the console — deliberately, because *"a wrong icon is worse
 * than a missing one"* — and there is no console gate in the browser tier. So a `size="large"`
 * button naming an icon drawn only at 20 is an invisible blank square in a toolbar, which is
 * exactly the defect this suite found twice on the day it was written.
 */

// ── the census ───────────────────────────────────────────────────────────────

interface CensusRow {
  readonly app: string;
  readonly tabSet: string;
  readonly tab: string;
  readonly group: string;
  readonly controls: number;
  readonly inScope: boolean;
}

/**
 * The committed command surface, parsed.
 *
 * The file lives outside `ui/` and only the two ribbon suites read it. That is deliberate:
 * `dev/ribbons/census.ts` is a **transcription**, and a transcription nobody checks is the thing
 * `storyConventions` warns about — *"a list nobody checks is worse than no list, because a reader
 * trusts it"*. There is no skip: the file is committed, and its absence is a finding.
 */
function census(): readonly CensusRow[] {
  const path = resolve(import.meta.dirname, '../..', ribbonCensusSource.file);
  const rows: CensusRow[] = [];
  for (const line of readFileSync(path, 'utf8').split('\n').slice(1)) {
    const [app, tabSet, tab, group, controls, inScope] = line.split('\t');
    if (app === undefined || tabSet === undefined || tab === undefined) continue;
    if (group === undefined || controls === undefined) continue;
    rows.push({ app, tabSet, tab, group, controls: Number(controls), inScope: inScope?.trim() === '1' });
  }
  return rows;
}

const rows = census();

/** Every tab of every application — core, view, File and contextual — for the gates that hold all of them to one rule. */
const everyTabByApplication: Readonly<Record<RibbonApplication, readonly RibbonTabEntry[]>> = {
  word: everyRibbonTab('word'),
  powerpoint: everyRibbonTab('powerpoint'),
  excel: everyRibbonTab('excel'),
};

/** `GroupFont 43` strings for one tab of one application, sorted. */
function expectedGroups(application: RibbonApplication, tab: RibbonTabEntry): string[] {
  const app = ribbonCensusSource.app[application];
  const source = tab.source;
  if (source.kind === 'core' || source.kind === 'contextual') {
    // A contextual tab is addressed by its set *and* its tab: a row is only this tab's when both columns agree.
    const tabSet = source.kind === 'core' ? ribbonCensusSource.coreTabSet : source.tabSet;
    return rows
      .filter(
        (row) =>
          row.app === app &&
          row.tabSet === tabSet &&
          row.tab === source.tab &&
          row.inScope,
      )
      .map((row) => `${row.group} ${String(row.controls)}`)
      .sort();
  }
  // The File tab: decision 1 of the approved plan makes it an ordinary ribbon tab, so its *groups*
  // are the backstage *destinations* and a group's id names a census tab. Its control count is
  // therefore the sum of that destination's own groups.
  const byTab = new Map<string, number>();
  for (const row of rows) {
    if (row.app !== app) continue;
    if (row.tabSet !== ribbonCensusSource.backstageTabSet) continue;
    if (!row.inScope) continue;
    byTab.set(row.tab, (byTab.get(row.tab) ?? 0) + row.controls);
  }
  return [...byTab].map(([id, controls]) => `${id} ${String(controls)}`).sort();
}

/**
 * **What is wrong with one tab's group identity against the census** — a core, File or contextual tab alike.
 *
 * A pure function so `the contextual rules can reject` can watch it refuse a doctored entry: a contextual tab is
 * addressed by its set and its tab, and a rule only ever shown a real tab might be matching nothing.
 */
export function groupIdentityFindings(application: RibbonApplication, tab: RibbonTabEntry): string[] {
  const expected = expectedGroups(application, tab);
  if (expected.length === 0) {
    return [`the census has no in-scope rows for ${application}/${tab.id}, which cannot be right`];
  }
  const declared = tab.groups.map((group) => `${group.id} ${String(group.controls)}`).sort();
  if (declared.join('\n') === expected.join('\n')) return [];
  return [
    `dev/ribbons/census.ts has drifted from ${ribbonCensusSource.file} for ${application}/${tab.id}: it ` +
      `declares [${declared.join(', ')}] and the census has [${expected.join(', ')}]. The declaration is a ` +
      'transcription and the TSV is the source; a ribbon whose groups are no longer Office’s groups is a ' +
      'ribbon somebody invented.',
  ];
}

describe('the ribbon census', () => {
  it('has rows at all, which is the assertion every other one here rests on', () => {
    expect(rows.length, `${ribbonCensusSource.file} parsed to nothing`).toBeGreaterThan(1000);
    expect(
      rows.some((row) => row.tabSet === ribbonCensusSource.coreTabSet),
      'no core-tab rows, so every comparison below would be empty against empty',
    ).toBe(true);
  });

  // Every tab, contextual tabs included: a contextual tab's groups are held to its `TabSet*` rows exactly as a core
  // tab's are held to its `None (Core Tab)` rows.
  for (const application of ribbonApplicationNames) {
    for (const tab of everyRibbonTab(application)) {
      it(`${application}/${tab.id} names exactly the groups the census marks in scope`, () => {
        expect(groupIdentityFindings(application, tab)).toEqual([]);
      });
    }
  }

  /**
   * ⚠ **Group *identity* only, and the omission is deliberate.**
   *
   * `tests/ribbon.test.ts` additionally asserts that `dev/word-tab-home.ts` *renders* exactly as
   * many controls as the census counts, padding each group with numbered filler to reach it. That
   * artefact is the **stress** specimen and the count is its whole point: 152 controls is the load
   * the collapse ladder has to survive.
   *
   * These modules are the **design** catalogue, and decision 3 of the approved plan is *"name what
   * the tab shows; menus stay shallow"* — a long tail that lives inside a dropdown in Office is
   * represented by that dropdown carrying a handful of real entries, so ~400 named commands stand
   * for ~1,880. Padding to the census count here would produce a ribbon nobody could audit and
   * would contradict the decision. `controls` is carried as data so the count is *visible* and
   * checked against the source; what is not asserted is that anything renders that many, and
   * `dev/word-tab-home.ts` remains the artefact that does.
   */
  it('declares each group’s census count as data, and nothing renders that many', () => {
    const withCommands = ribbonApplicationNames.flatMap((application) =>
      everyRibbonTab(application).flatMap((tab) =>
        tab.groups.filter((group) => group.commands !== undefined),
      ),
    );
    expect(
      withCommands.length,
      'no group has authored commands, so the claim below is about an empty set',
    ).toBeGreaterThan(0);
    expect(
      withCommands.some((group) => (group.commands ?? []).length < group.controls),
      'every authored group renders at least its census count, which means the modules are ' +
        'padding after all — see this test’s own comment on why they must not.',
    ).toBe(true);
  });

  it('gives every tab a kebab id and a real label, and distinct ids within an application', () => {
    for (const application of ribbonApplicationNames) {
      const ids = everyRibbonTab(application).map((tab) => tab.id);
      expect(new Set(ids).size, `${application} declares a tab id twice`).toBe(ids.length);
      for (const tab of everyRibbonTab(application)) {
        expect(tab.id, `${application}/${tab.id} is not kebab case`).toMatch(/^[a-z][a-z0-9-]*$/);
        expect(tab.label.trim(), `${application}/${tab.id} has no label`).not.toBe('');
        expect(tabAppearanceNames).toContain(tab.appearance);
        expect(tab.groups.length, `${application}/${tab.id} has no groups`).toBeGreaterThan(0);
      }
      const sets = ribbonContextualSets[application];
      expect(new Set(sets.map((set) => set.id)).size, `${application} declares a contextual set id twice`).toBe(
        sets.length,
      );
      for (const set of sets) {
        expect(set.id, `${application}/${set.id} is not kebab case`).toMatch(/^[a-z][a-z0-9-]*$/);
        expect(set.label.trim(), `${application}/${set.id} has no label`).not.toBe('');
        expect(set.tabs.length, `${application}/${set.id} has no tabs`).toBeGreaterThan(0);
      }
    }
  });

  it('declares every in-scope core tab the census carries, not a chosen subset', () => {
    for (const application of ribbonApplicationNames) {
      const app = ribbonCensusSource.app[application];
      const fromCensus = [
        ...new Set(
          rows
            .filter(
              (row) =>
                row.app === app && row.tabSet === ribbonCensusSource.coreTabSet && row.inScope,
            )
            .map((row) => row.tab),
        ),
      ].sort();
      const declared = everyRibbonTab(application)
        .filter((tab) => tab.source.kind === 'core')
        .map((tab) => (tab.source.kind === 'core' ? tab.source.tab : ''))
        .sort();
      expect(
        declared,
        `${application} skips an in-scope core tab. The scaffold declares every tab from unit 0 ` +
          'precisely so this can be a plain equality and there is no gap to keep a ledger of.',
      ).toEqual(fromCensus);
    }
  });
});

// ── the contextual tab sets ──────────────────────────────────────────────────

/** The census's in-scope `TabSet*` tabs for one application, as sorted `<set> <tab>` strings. */
function contextualTabsInCensus(application: RibbonApplication): string[] {
  const app = ribbonCensusSource.app[application];
  return [
    ...new Set(
      rows
        .filter(
          (row) =>
            row.app === app && row.tabSet.startsWith(ribbonCensusSource.contextualTabSetPrefix) && row.inScope,
        )
        .map((row) => `${row.tabSet} ${row.tab}`),
    ),
  ].sort();
}

/**
 * **What is wrong with one application's contextual sets, against the census.**
 *
 * The user's decision builds four sets, so most of the census's contextual tabs are *not* declared, and a gate that
 * checked only the declared ones would accept a set quietly dropped or never considered. So every in-scope `TabSet*`
 * tab is accounted for **exactly once** — declared under a built set, recorded in that set's `unbuiltTabs`, or
 * recorded in an unbuilt set — and six things are refused:
 *
 * 1. an in-scope census tab that is neither built nor recorded;
 * 2. a tab accounted for twice, in any combination of the three places;
 * 3. a declared or recorded tab the census carries no in-scope rows for;
 * 4. a built tab whose `source` is not contextual, or names a set other than the one it is declared under;
 * 5. a built tab whose appearance is not `contextual`;
 * 6. a record with no reason.
 */
export function contextualCoverageFindings(
  application: RibbonApplication,
  built: readonly RibbonContextualSetEntry[],
  unbuilt: readonly UnbuiltContextualSet[],
): string[] {
  const findings: string[] = [];
  const accounted: string[] = [];
  for (const set of built) {
    for (const tab of set.tabs) {
      if (tab.appearance !== 'contextual') {
        findings.push(`${application}/${tab.id} is declared in ${set.tabSet} and its appearance is ${tab.appearance}.`);
      }
      if (tab.source.kind !== 'contextual') {
        findings.push(`${application}/${tab.id} is declared in ${set.tabSet} and its source is ${tab.source.kind}.`);
        continue;
      }
      if (tab.source.tabSet !== set.tabSet) {
        findings.push(
          `${application}/${tab.id} is declared in ${set.tabSet} and its source names ${tab.source.tabSet}.`,
        );
      }
      accounted.push(`${tab.source.tabSet} ${tab.source.tab}`);
    }
    for (const record of set.unbuiltTabs) {
      if (record.reason.trim() === '') {
        findings.push(`${application}: ${set.tabSet} ${record.tab} is recorded as unbuilt with no reason.`);
      }
      accounted.push(`${set.tabSet} ${record.tab}`);
    }
  }
  for (const set of unbuilt) {
    if (set.reason.trim() === '') findings.push(`${application}: ${set.tabSet} is recorded as unbuilt with no reason.`);
    for (const tab of set.tabs) accounted.push(`${set.tabSet} ${tab}`);
  }
  const inCensus = contextualTabsInCensus(application);
  const counts = new Map<string, number>();
  for (const key of accounted) counts.set(key, (counts.get(key) ?? 0) + 1);
  for (const [key, count] of counts) {
    if (count > 1) {
      findings.push(`${application}: ${key} is accounted for ${String(count)} times. A tab is built or recorded, once.`);
    }
    if (!inCensus.includes(key)) {
      findings.push(`${application}: ${key} has no in-scope rows in ${ribbonCensusSource.file}.`);
    }
  }
  for (const key of inCensus) {
    if (!counts.has(key)) {
      findings.push(`${application}: ${key} is in scope in the census and is neither built nor recorded as unbuilt.`);
    }
  }
  return findings;
}

describe('the contextual tab sets', () => {
  it('reads contextual rows at all, or every comparison below is empty against empty', () => {
    for (const application of ribbonApplicationNames) {
      expect(contextualTabsInCensus(application).length, application).toBeGreaterThan(15);
    }
  });

  for (const application of ribbonApplicationNames) {
    it(`${application} builds or records every in-scope contextual tab the census carries, each exactly once`, () => {
      expect(
        contextualCoverageFindings(application, ribbonContextualSets[application], unbuiltContextualSets[application]),
      ).toEqual([]);
    });
  }

  it('builds exactly the four common sets the user decided on, with exactly their tabs', () => {
    const built = Object.fromEntries(
      ribbonApplicationNames.map((application) => [
        application,
        ribbonContextualSets[application].map(
          (set) => `${set.label}: ${set.tabs.map((tab) => tab.label).join(', ')}`,
        ),
      ]),
    );
    const four = [
      'Table Tools: Table Design, Layout',
      'Picture Tools: Picture Format',
      'Drawing Tools: Shape Format',
      'Chart Tools: Chart Design, Format',
    ];
    expect(built).toEqual({
      word: four,
      powerpoint: four,
      excel: ['Table Tools: Table Design', ...four.slice(1)],
    });
  });

  it('addresses each built tab to the census tab the decision names', () => {
    const sources = Object.fromEntries(
      ribbonApplicationNames.map((application) => [
        application,
        ribbonContextualSets[application].flatMap((set) =>
          set.tabs.map((tab) =>
            tab.source.kind === 'contextual' ? `${tab.id} ${tab.source.tabSet} ${tab.source.tab}` : tab.id,
          ),
        ),
      ]),
    );
    const common = [
      'picture-format TabSetPictureTools TabPictureToolsFormat',
      'shape-format TabSetDrawingTools TabDrawingToolsFormat',
      'chart-design TabSetChartTools TabChartToolsDesignNew',
      'chart-format TabSetChartTools TabChartToolsFormatNew',
    ];
    const tables = [
      'table-design TabSetTableTools TabTableToolsDesign',
      'table-layout TabSetTableTools TabTableToolsLayout',
    ];
    expect(sources).toEqual({
      word: [...tables, ...common],
      powerpoint: [...tables, ...common],
      excel: ['table-design TabSetTableToolsExcel TabTableToolsDesignExcel', ...common],
    });
  });

  it('records every unbuilt set with the user’s reason, and only Chart Tools’ three older tabs with another', () => {
    expect(unbuiltContextualSetReason).toBe(
      'Only the four common sets (Table, Picture, Drawing, Chart) are built; decided by the user, 2026-09-15.',
    );
    for (const application of ribbonApplicationNames) {
      expect(unbuiltContextualSets[application].length, application).toBeGreaterThan(10);
      for (const set of unbuiltContextualSets[application]) {
        expect(set.reason, `${application}/${set.tabSet}`).toBe(unbuiltContextualSetReason);
      }
      const inBuiltSets = ribbonContextualSets[application].flatMap((set) =>
        set.unbuiltTabs.map((record) => `${set.tabSet} ${record.tab} ${record.reason === legacyChartTabReason ? 'legacy' : record.reason}`),
      );
      expect(inBuiltSets, application).toEqual([
        'TabSetChartTools TabChartToolsDesign legacy',
        'TabSetChartTools TabChartToolsFormat legacy',
        'TabSetChartTools TabChartToolsLayout legacy',
      ]);
    }
  });

  it('keeps contextual tabs out of the core strip, and core tabs out of the sets', () => {
    for (const application of ribbonApplicationNames) {
      for (const tab of ribbonCensus[application]) {
        expect(tab.appearance, `${application}/${tab.id}`).not.toBe('contextual');
        expect(tab.source.kind, `${application}/${tab.id}`).not.toBe('contextual');
      }
    }
  });
});

describe('the contextual rules can reject', () => {
  const word = ribbonContextualSets.word;
  const unbuiltWord = unbuiltContextualSets.word;
  const tableDesign = everyRibbonTab('word').find((tab) => tab.id === 'table-design');
  const pictureFormat = everyRibbonTab('word').find((tab) => tab.id === 'picture-format');

  it('starts from green: the real entries pass both rules', () => {
    expect(tableDesign).toBeDefined();
    expect(pictureFormat).toBeDefined();
    expect(contextualCoverageFindings('word', word, unbuiltWord)).toEqual([]);
    if (tableDesign !== undefined) expect(groupIdentityFindings('word', tableDesign)).toEqual([]);
  });

  it('refuses a contextual tab that has lost a group, or is held to another application’s rows', () => {
    if (tableDesign === undefined) throw new Error('word declares no table-design tab');
    expect(groupIdentityFindings('word', { ...tableDesign, groups: tableDesign.groups.slice(1) })).toEqual([
      expect.stringContaining('has drifted'),
    ]);
    // PowerPoint's Table Design has the same set and tab ids and different groups.
    expect(groupIdentityFindings('powerpoint', tableDesign)).toEqual([expect.stringContaining('has drifted')]);
  });

  it('refuses the right tab id in the wrong set, which a rule matching on the tab alone would accept', () => {
    if (pictureFormat === undefined) throw new Error('word declares no picture-format tab');
    const moved: RibbonTabEntry = {
      ...pictureFormat,
      source: { kind: 'contextual', tabSet: 'TabSetPictureToolsClassic', tab: 'TabPictureToolsFormat' },
    };
    expect(groupIdentityFindings('word', moved)).toEqual([expect.stringContaining('no in-scope rows')]);
  });

  it('refuses a set that is neither built nor recorded', () => {
    const findings = contextualCoverageFindings('word', word, unbuiltWord.slice(1));
    expect(findings).toEqual([
      expect.stringContaining('TabSet3DModelTools Tab3DModelToolsFormat is in scope in the census and is neither built'),
    ]);
  });

  it('refuses a built tab recorded as unbuilt too', () => {
    const twice: UnbuiltContextualSet = {
      tabSet: 'TabSetTableTools',
      label: 'Table Tools',
      tabs: ['TabTableToolsDesign'],
      reason: unbuiltContextualSetReason,
    };
    expect(contextualCoverageFindings('word', word, [...unbuiltWord, twice])).toEqual([
      expect.stringContaining('TabSetTableTools TabTableToolsDesign is accounted for 2 times'),
    ]);
  });

  it('refuses a record of a tab the census does not carry in scope', () => {
    // Excel's Power Query editor is in the census, and every one of its rows is out of scope.
    const outOfScope: UnbuiltContextualSet = {
      tabSet: 'TabSetPowerQueryEdit',
      label: 'Power Query',
      tabs: ['TabPowerQueryQueryEdit'],
      reason: unbuiltContextualSetReason,
    };
    expect(
      contextualCoverageFindings('excel', ribbonContextualSets.excel, [...unbuiltContextualSets.excel, outOfScope]),
    ).toEqual([expect.stringContaining('TabSetPowerQueryEdit TabPowerQueryQueryEdit has no in-scope rows')]);
  });

  it('refuses a built tab declared under a set its source does not name, or with the wrong appearance', () => {
    const [tableTools, ...rest] = word;
    if (tableTools === undefined) throw new Error('word builds no sets');
    expect(contextualCoverageFindings('word', [{ ...tableTools, tabSet: 'TabSetTableToolsExcel' }, ...rest], unbuiltWord)).toEqual([
      expect.stringContaining('is declared in TabSetTableToolsExcel and its source names TabSetTableTools'),
      expect.stringContaining('is declared in TabSetTableToolsExcel and its source names TabSetTableTools'),
    ]);
    const [first, ...others] = tableTools.tabs;
    if (first === undefined) throw new Error('word table-tools has no tabs');
    expect(
      contextualCoverageFindings('word', [{ ...tableTools, tabs: [{ ...first, appearance: 'always' }, ...others] }, ...rest], unbuiltWord),
    ).toEqual([expect.stringContaining('its appearance is always')]);
  });

  it('refuses a record with no reason', () => {
    const [firstSet, ...others] = unbuiltWord;
    if (firstSet === undefined) throw new Error('word records no unbuilt sets');
    expect(contextualCoverageFindings('word', word, [{ ...firstSet, reason: ' ' }, ...others])).toEqual([
      expect.stringContaining('recorded as unbuilt with no reason'),
    ]);
  });
});

// ── the priority rubric ──────────────────────────────────────────────────────

/**
 * The rubric of `dev/ribbons/census.ts`'s header, as something that can reject.
 *
 * Written as a pure function over one tab rather than as three assertions, so the instrument tests
 * below can watch it refuse a hand-made bad entry. A rule that has never been seen to fire is a
 * rule that might be matching nothing — this catalogue's oldest lesson, and with ~230 groups the
 * one place it would be least visible.
 */
export function rubricFindings(tab: RibbonTabEntry): string[] {
  const findings: string[] = [];
  const primary = tab.groups.filter((group) => group.priority === 'primary');
  if (primary.length > 2) {
    findings.push(
      `${tab.id} declares ${String(primary.length)} primary groups ` +
        `(${primary.map((group) => group.label).join(', ')}). primary means "the group the tab ` +
        'exists for"; three of them means none of them.',
    );
  }
  for (const group of tab.groups) {
    if (!group.inScope && group.priority !== 'ancillary') {
      findings.push(
        `${tab.id}/${group.id} is out of scope and ${group.priority}. A group the census does ` +
          'not carry into this project is never anybody’s reason for opening a tab.',
      );
    }
    if (group.controls <= 2 && (group.priority === 'primary' || group.priority === 'standard')) {
      findings.push(
        `${tab.id}/${group.id} holds ${String(group.controls)} controls and is ` +
          `${group.priority}. A group of two controls or fewer gives way early — secondary if its ` +
          'commands are reachable elsewhere, ancillary otherwise.',
      );
    }
  }
  return findings;
}

describe('the priority rubric', () => {
  it('is followed by every tab of every application', () => {
    const findings = ribbonApplicationNames.flatMap((application) =>
      everyRibbonTab(application).flatMap((tab) =>
        rubricFindings(tab).map((finding) => `${application}/${finding}`),
      ),
    );
    expect(findings).toEqual([]);
  });

  it('uses only priorities the ribbon model declares', () => {
    for (const application of ribbonApplicationNames) {
      for (const tab of everyRibbonTab(application)) {
        for (const group of tab.groups) {
          expect(groupPriorityNames).toContain(group.priority);
        }
      }
    }
  });

  it('gives every tab at least one primary or standard group, so a whole tab cannot give way at once', () => {
    for (const application of ribbonApplicationNames) {
      for (const tab of everyRibbonTab(application)) {
        expect(
          tab.groups.some(
            (group) => group.priority === 'primary' || group.priority === 'standard',
          ),
          `${application}/${tab.id} is entirely secondary and ancillary, so at 1000px it would be ` +
            'a strip of collapsed buttons and nothing else.',
        ).toBe(true);
      }
    }
  });

  it('draws a placeholder at the strongest priority its tab declares, not at a constant', () => {
    // The property `placeholderTab` rests on: a tab that will hold a primary group when its unit
    // lands must not collapse earlier today than it will then.
    for (const application of ribbonApplicationNames) {
      for (const tab of everyRibbonTab(application)) {
        const strongest = strongestPriority(tab);
        const index = groupPriorityNames.indexOf(strongest);
        for (const group of tab.groups) {
          expect(
            groupPriorityNames.indexOf(group.priority),
            `${application}/${tab.id} has a ${group.priority} group but its placeholder would ` +
              `be drawn ${strongest}`,
          ).toBeGreaterThanOrEqual(index);
        }
      }
    }
  });
});

describe('the rubric can reject', () => {
  const group = (over: Partial<RibbonGroupEntry>): RibbonGroupEntry => ({
    id: 'GroupX',
    label: 'X',
    priority: 'standard',
    controls: 12,
    inScope: true,
    ...over,
  });
  const tab = (...groups: RibbonGroupEntry[]): RibbonTabEntry => ({
    id: 'specimen',
    label: 'Specimen',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabSpecimen' },
    groups,
  });

  it('accepts a tab that follows it', () => {
    expect(
      rubricFindings(
        tab(
          group({ id: 'A', priority: 'primary' }),
          group({ id: 'B', priority: 'primary' }),
          group({ id: 'C' }),
          group({ id: 'D', priority: 'secondary', controls: 2 }),
          group({ id: 'E', priority: 'ancillary', controls: 1 }),
        ),
      ),
    ).toEqual([]);
  });

  it('refuses a third primary group', () => {
    const findings = rubricFindings(
      tab(
        group({ id: 'A', priority: 'primary' }),
        group({ id: 'B', priority: 'primary' }),
        group({ id: 'C', priority: 'primary' }),
      ),
    );
    expect(findings).toHaveLength(1);
    expect(findings[0]).toContain('3 primary groups');
  });

  it('refuses an out-of-scope group that is not ancillary', () => {
    const findings = rubricFindings(tab(group({ id: 'Copilot', inScope: false })));
    expect(findings).toHaveLength(1);
    expect(findings[0]).toContain('out of scope');
  });

  it('refuses a two-control group that does not give way early', () => {
    expect(rubricFindings(tab(group({ controls: 2, priority: 'primary' })))).toHaveLength(1);
    expect(rubricFindings(tab(group({ controls: 1, priority: 'standard' })))).toHaveLength(1);
    expect(rubricFindings(tab(group({ controls: 2, priority: 'secondary' })))).toEqual([]);
    expect(rubricFindings(tab(group({ controls: 3, priority: 'standard' })))).toEqual([]);
  });
});

// ── the icons ────────────────────────────────────────────────────────────────

/** Which drawing a control of this size asks `<mjx-icon>` for. Read, never restated. */
function iconSizeFor(size: ControlSize | undefined): number {
  return controlSizes[size ?? 'small'].iconSize;
}

/** Every variant a command needs: a toggle draws filled when pressed, a verb never does. */
function variantsFor(command: RibbonCommand): readonly string[] {
  return command.toggle === true ? ['regular', 'filled'] : ['regular'];
}

/** What is missing from `src/icons/manifest.ts` for one command, in a message a person can act on. */
export function iconFindings(command: RibbonCommand): string[] {
  if (command.icon === undefined) {
    // `size: 'icon'` draws the label off-screen and the glyph alone, so with no glyph it draws
    // nothing at all. Unit 2b added the first toggle with no icon — PowerPoint's Text Shadow — and
    // `toggle()`'s own default size is `icon`, which is exactly how that would have happened.
    return command.size === 'icon'
      ? [
          `${command.id} is size="icon" and names no icon, so it draws an empty square with its ` +
            'name off-screen. A command with no glyph is `small`, and its label is the command.',
        ]
      : [];
  }
  const request = iconRequests.find((entry) => entry.name === command.icon);
  if (request === undefined) {
    return [
      `${command.id} names the icon '${command.icon}', which src/icons/manifest.ts does not ` +
        'request at all. Add a row and run `npm run icons:subset`; never hand-edit generated.ts.',
    ];
  }
  const size = iconSizeFor(command.size);
  const findings: string[] = [];
  if (!(request.sizes as readonly number[]).includes(size)) {
    findings.push(
      `${command.id} draws '${command.icon}' at ${String(size)}px (size="${command.size ?? 'small'}") ` +
        `and the subset carries ${request.sizes.join(', ')}. <mjx-icon> renders nothing for a ` +
        'drawing it lacks and only reports it on the console, so this is a blank square in a ' +
        'toolbar that no gate in the browser tier can see.',
    );
  }
  for (const variant of variantsFor(command)) {
    if (!(request.variants as readonly string[]).includes(variant)) {
      findings.push(
        `${command.id} needs '${command.icon}' in the ${variant} variant and the subset carries ` +
          `${request.variants.join(', ')}. A toggle draws filled when it is pressed, so a ` +
          'regular-only request is a control that goes blank the moment somebody presses it.',
      );
    }
  }
  return findings;
}

describe('every icon a ribbon command names can actually be drawn', () => {
  const commands = everyRibbonCommand();

  it('sweeps a non-empty set, or it asserts nothing at all', () => {
    expect(commands.length).toBeGreaterThan(20);
    expect(
      commands.filter((command) => command.icon !== undefined).length,
      'no command in dev/ribbons/ names an icon, so the sweep below is vacuous',
    ).toBeGreaterThan(15);
  });

  it('resolves every name, size and variant against the committed subset', () => {
    expect(commands.flatMap(iconFindings)).toEqual([]);
  });

  it('gives every command a unique id, because a host binds its override by one', () => {
    const ids = commands.map((command) => command.id);
    expect(new Set(ids).size, 'two commands share an id, so one override would replace both').toBe(
      ids.length,
    );
  });

  it('spells every id `<app>.<tab>.<group>.<command>` and names an application that exists', () => {
    for (const command of commands) {
      expect(command.id, `${command.id} is not four dotted, lower-case segments`).toMatch(
        /^[a-z]+(?:\.[a-z0-9-]+){3}$/,
      );
      expect(ribbonApplicationNames as readonly string[]).toContain(command.id.split('.')[0]);
    }
  });
});

describe('the icon rule can reject', () => {
  it('accepts a command the subset carries', () => {
    expect(
      iconFindings({ id: 'word.home.font.bold', label: 'Bold', icon: 'text-bold', toggle: true }),
    ).toEqual([]);
    expect(iconFindings({ id: 'word.home.clipboard.cut', label: 'Cut', icon: 'cut' })).toEqual([]);
  });

  it('accepts a command with no icon at all', () => {
    expect(iconFindings({ id: 'word.home.styles.gallery', label: 'Styles' })).toEqual([]);
    expect(
      iconFindings({ id: 'powerpoint.home.font.text-shadow', label: 'Text Shadow', size: 'small', toggle: true }),
    ).toEqual([]);
  });

  it('catches an icon-size command with no icon to draw', () => {
    const findings = iconFindings({
      id: 'powerpoint.home.font.text-shadow',
      label: 'Text Shadow',
      size: 'icon',
      toggle: true,
    });
    expect(findings).toHaveLength(1);
    expect(findings[0]).toContain('empty square');
  });

  it('catches an icon the manifest never requested', () => {
    const findings = iconFindings({ id: 'word.home.font.x', label: 'X', icon: 'not-an-icon' });
    expect(findings).toHaveLength(1);
    expect(findings[0]).toContain('does not request at all');
  });

  it('catches a large button whose icon has no 24px drawing', () => {
    // `table-checker` is drawn at 20 alone, and `large` asks for 24 — the exact shape of the defect
    // the ribbon programme hit with `folder-open`, and the reason this suite exists. It used to be
    // `table`, until unit 3 made Table the large headline of the Insert tab and requested its 24;
    // `table-checker` is the better specimen because the vendor ships no 24 of it at all, so no later
    // unit can quietly turn this refusal into an acceptance.
    const findings = iconFindings({
      id: 'excel.home.styles.format-as-table',
      label: 'Format as Table',
      icon: 'table-checker',
      size: 'large',
    });
    expect(findings).toHaveLength(1);
    expect(findings[0]).toContain('24px');
  });

  it('catches a toggle whose icon has no filled drawing', () => {
    // `search` is a one-shot verb in the subset and carries `regular` alone. Used as a toggle it
    // would vanish the moment it was pressed.
    const findings = iconFindings({
      id: 'word.home.editing.find',
      label: 'Find',
      icon: 'search',
      toggle: true,
    });
    expect(findings).toHaveLength(1);
    expect(findings[0]).toContain('filled variant');
  });
});

// ── the overrides the hosts bind ─────────────────────────────────────────────

/**
 * Every `'<app>.<tab>.<group>.<command>':` key an assembly source writes.
 *
 * ⚠ **This is the only thing standing between a typo and a control that silently disappears.**
 * `renderCommand` ignores an override whose id names no command — deliberately, because a host
 * binds by id and a tab's command list grows unit by unit, so an override written a unit early
 * must not be an exception in front of an auditor. The cost of that choice is that a *misspelt*
 * id is equally silent: the binding is dropped, the generic button is drawn in its place, and
 * nothing says so. Reading the keys back out of the source is what makes it loud again.
 */
function overrideKeysIn(source: string): string[] {
  return [...source.matchAll(/'([a-z]+(?:\.[a-z0-9-]+){3})'\s*:/g)].map((match) => match[1] ?? '');
}

/** The six files that assemble a ribbon and bind overrides over the census. */
const assemblySources = ['ribbons/word', 'ribbons/powerpoint', 'ribbons/excel', 'shell/word', 'shell/powerpoint', 'shell/excel'].map(
  (name) => ({
    name: `${name}.stories.ts`,
    source: readFileSync(resolve(import.meta.dirname, `../stories/${name}.stories.ts`), 'utf8'),
  }),
);

/** Every command id any host binds an override to. */
const overriddenIds = new Set(assemblySources.flatMap((file) => overrideKeysIn(file.source)));

describe('every override a host binds names a command that exists', () => {
  const files = assemblySources;
  const declared = new Set(everyRibbonCommand().map((command) => command.id));

  it('finds keys at all, or it is checking nothing', () => {
    const total = files.flatMap((file) => overrideKeysIn(file.source));
    expect(total.length, 'no override keys found in any assembly source').toBeGreaterThan(20);
  });

  it('reads a key the way a host writes one, and nothing else', () => {
    // The instrument test. A scanner that had stopped matching would make every assertion below
    // pass over an empty list, which is the failure this catalogue keeps meeting.
    expect(overrideKeysIn("{ 'word.home.font.name': html`<x>` }")).toEqual(['word.home.font.name']);
    expect(overrideKeysIn("{ 'word.home.font.name' : 1 }")).toEqual(['word.home.font.name']);
    expect(overrideKeysIn('a `word.home.font.name` in prose'), 'prose is not a binding').toEqual([]);
    expect(overrideKeysIn("{ 'word.home.font': 1 }"), 'three segments is not an id').toEqual([]);
  });

  for (const file of files) {
    it(`${file.name} binds only declared command ids`, () => {
      const unknown = overrideKeysIn(file.source).filter((key) => !declared.has(key));
      expect(
        unknown,
        `${file.name} binds an override to a command id dev/ribbons/census.ts does not declare. ` +
          'renderCommand ignores it silently and draws the generic control instead, so the ' +
          'binding is simply gone — this is the only place that says so.',
      ).toEqual([]);
    });
  }
});

// ── a split button whose face is a state ─────────────────────────────────────

/** One `'<id>': html`<tag …>`` binding, reduced to its tag and its opening tag's attribute text. */
export interface BoundControl {
  readonly key: string;
  readonly tag: string;
  readonly attributes: string;
}

/** Every binding in a host's source, with the element it opens with. */
export function boundControlsIn(source: string): BoundControl[] {
  return [...source.matchAll(/'([a-z]+(?:\.[a-z0-9-]+){3})'\s*:\s*html`<(mjx-[a-z-]+)([^>]*)>/g)].map(
    (match) => ({ key: match[1] ?? '', tag: match[2] ?? '', attributes: match[3] ?? '' }),
  );
}

/**
 * **What is wrong with a host's split-button bindings, against the census's `toggle` and `pressed`.**
 *
 * A census command that is a state (`toggle: true`) and that a host binds as `<mjx-split-button>` is
 * Office's *state with a menu*, and it must opt in with `toggle` — otherwise its face fires an action
 * and never draws pressed, which is the gap Track Changes, Show Comments, Hide Ink and Eraser shipped
 * with. The converse is refused too, so a split cannot claim a state the census does not declare, and
 * `pressed="true"` must agree with the census's starting position. Both hosts are held to one table,
 * so the catalogue and the shell cannot start a command in different positions.
 */
export function splitToggleFindings(
  file: { readonly name: string; readonly source: string },
  commands: readonly RibbonCommand[],
): string[] {
  const byId = new Map(commands.map((command) => [command.id, command]));
  const findings: string[] = [];
  for (const bound of boundControlsIn(file.source)) {
    if (bound.tag !== 'mjx-split-button') continue;
    const command = byId.get(bound.key);
    if (command === undefined) continue;
    const toggles = /(?:^|\s)toggle(?:\s|$)/.test(bound.attributes);
    const pressed = /(?:^|\s)pressed="true"/.test(bound.attributes);
    const isState = command.toggle === true;
    if (isState && !toggles) {
      findings.push(
        `${file.name}: ${bound.key} is a toggle in dev/ribbons/census.ts and is bound as a ` +
          '<mjx-split-button> without `toggle`, so its face fires an action and never draws pressed.',
      );
    }
    if (!isState && toggles) {
      findings.push(
        `${file.name}: ${bound.key} is bound as <mjx-split-button toggle>, and the census does not ` +
          'declare it a toggle. Declare `toggle: true` there, or drop `toggle` here.',
      );
    }
    if (toggles && pressed !== (command.pressed === true)) {
      findings.push(
        `${file.name}: ${bound.key} starts ${pressed ? 'pressed' : 'unpressed'} here and ` +
          `${command.pressed === true ? 'pressed' : 'unpressed'} in the census.`,
      );
    }
  }
  return findings;
}

describe('a split button whose face is a state draws pressed, in every host', () => {
  const commands = everyRibbonCommand();

  it('finds toggling split buttons at all, or it is checking nothing', () => {
    const toggling = assemblySources
      .flatMap((file) => boundControlsIn(file.source))
      .filter((bound) => bound.tag === 'mjx-split-button' && /(?:^|\s)toggle(?:\s|$)/.test(bound.attributes));
    // Word's Eraser, Track Changes, Show Comments and Hide Ink, PowerPoint's Eraser, Show Comments and Hide
    // Ink, and Excel's Hide Ink, each in two hosts.
    expect(toggling.map((bound) => bound.key).sort()).toEqual(
      [
        'excel.review.ink.hide-ink',
        'excel.review.ink.hide-ink',
        'powerpoint.draw.write.eraser',
        'powerpoint.draw.write.eraser',
        'powerpoint.review.comments.show-comments',
        'powerpoint.review.comments.show-comments',
        'powerpoint.review.ink.hide-ink',
        'powerpoint.review.ink.hide-ink',
        'word.draw.write.eraser',
        'word.draw.write.eraser',
        'word.review.comments.show-comments',
        'word.review.comments.show-comments',
        'word.review.ink.hide-ink',
        'word.review.ink.hide-ink',
        'word.review.tracking.track-changes',
        'word.review.tracking.track-changes',
      ].sort(),
    );
  });

  for (const file of assemblySources) {
    it(`${file.name} binds every census toggle it draws as a split with \`toggle\``, () => {
      expect(splitToggleFindings(file, commands)).toEqual([]);
    });
  }
});

describe('the split toggle rule can reject', () => {
  const state: RibbonCommand = { id: 'word.review.tracking.track-changes', label: 'Track Changes', toggle: true };
  const on: RibbonCommand = { id: 'word.review.comments.show-comments', label: 'Show Comments', toggle: true, pressed: true };
  const verb: RibbonCommand = { id: 'word.home.clipboard.paste', label: 'Paste' };
  const host = (key: string, attributes: string): { name: string; source: string } => ({
    name: 'fixture.stories.ts',
    source: `const bindings = {\n  '${key}': html\`<mjx-split-button\n    ${attributes}\n    label="x"\n    @mjx-menu-request=\${openDeclaredSurface}\n  ></mjx-split-button>\`,\n};\n`,
  });

  it('reads a binding the way a host writes one', () => {
    expect(boundControlsIn(host(state.id, 'toggle').source)).toEqual([
      { key: state.id, tag: 'mjx-split-button', attributes: '\n    toggle\n    label="x"\n    @mjx-menu-request=${openDeclaredSurface}\n  ' },
    ]);
  });

  it('accepts a declared state bound with toggle, in its declared position', () => {
    expect(splitToggleFindings(host(state.id, 'toggle'), [state])).toEqual([]);
    expect(splitToggleFindings(host(on.id, 'toggle\n    pressed="true"'), [on])).toEqual([]);
    expect(splitToggleFindings(host(verb.id, 'size="large"'), [verb])).toEqual([]);
  });

  it('refuses a declared state whose split forgot toggle', () => {
    const findings = splitToggleFindings(host(state.id, 'size="large"'), [state]);
    expect(findings).toHaveLength(1);
    expect(findings[0]).toContain('without `toggle`');
  });

  it('refuses a split that claims a state the census does not declare', () => {
    const findings = splitToggleFindings(host(verb.id, 'toggle'), [verb]);
    expect(findings).toHaveLength(1);
    expect(findings[0]).toContain('does not declare it a toggle');
  });

  it('refuses a starting position that disagrees with the census, in either direction', () => {
    expect(splitToggleFindings(host(on.id, 'toggle'), [on])[0]).toContain('starts unpressed here');
    expect(splitToggleFindings(host(state.id, 'toggle\n    pressed="true"'), [state])[0]).toContain(
      'starts pressed here',
    );
  });

  it('does not mistake `toggle` inside another attribute for the opt-in', () => {
    expect(splitToggleFindings(host(state.id, 'data-toggle="x"'), [state])[0]).toContain('without `toggle`');
  });
});

// ── exclusive sets ───────────────────────────────────────────────────────────

/** One member of an exclusive set, with where it is declared. */
export interface ExclusiveSetMember {
  readonly application: string;
  readonly tab: string;
  readonly command: RibbonCommand;
}

/** Every exclusive set a census declares, by name, with its members in declared order. */
export function exclusiveSetsIn(
  census: Readonly<Partial<Record<RibbonApplication, readonly RibbonTabEntry[]>>>,
): Map<string, ExclusiveSetMember[]> {
  const sets = new Map<string, ExclusiveSetMember[]>();
  for (const [application, tabs] of Object.entries(census)) {
    for (const tab of tabs) {
      for (const group of tab.groups) {
        for (const command of group.commands ?? []) {
          if (command.exclusive === undefined) continue;
          const members = sets.get(command.exclusive) ?? [];
          members.push({ application, tab: tab.id, command });
          sets.set(command.exclusive, members);
        }
      }
    }
  }
  return sets;
}

/**
 * **What is wrong with a census's exclusive sets.**
 *
 * `src/controls/exclusive-set.ts` keeps one member pressed once somebody presses one, and cannot supply the
 * first: a set that starts with nothing pressed draws a ribbon where Word is in no view, and a set that
 * starts with two draws the defect this mechanism removed. So **exactly one member starts pressed**. Every
 * member must be a toggle, because a button has no position to release, and every member must live in one
 * tab of one application, because the coordinator looks the set up in the pressed member's tab and a
 * member elsewhere would never be released. A set of one is a toggle that cannot be released at all.
 *
 * **A set that declares `exclusiveAllowsNone` starts with at most one pressed instead**, because a press can
 * empty it, and every member declares it or none does: the coordinator reads it from the member pressed, so
 * a set that disagreed would keep or release its holder by which one somebody pressed. `exclusiveAllowsNone`
 * on a command in no set declares nothing, and is refused.
 */
export function exclusiveSetFindings(
  census: Readonly<Partial<Record<RibbonApplication, readonly RibbonTabEntry[]>>>,
): string[] {
  const findings: string[] = [];
  for (const tabs of Object.values(census)) {
    for (const tab of tabs) {
      for (const group of tab.groups) {
        for (const command of group.commands ?? []) {
          if (command.exclusiveAllowsNone === true && command.exclusive === undefined) {
            findings.push(`${command.id} declares exclusiveAllowsNone and is in no set, so it declares nothing.`);
          }
        }
      }
    }
  }
  for (const [set, members] of exclusiveSetsIn(census)) {
    const places = new Set(members.map((member) => `${member.application} ${member.tab}`));
    if (places.size > 1) {
      findings.push(
        `${set} has members in ${[...places].sort().join(' and ')}. A set is looked up in the pressed ` +
          "member's tab, so a member elsewhere is never released.",
      );
    }
    if (members.length < 2) {
      findings.push(`${set} has one member, which could never be released. Drop \`exclusive\` from it.`);
    }
    for (const { command } of members) {
      if (command.toggle !== true) {
        findings.push(`${command.id} is in ${set} and is not a toggle, so it holds no position to release.`);
      }
    }
    const allowingNone = members.filter((member) => member.command.exclusiveAllowsNone === true).length;
    if (allowingNone !== 0 && allowingNone !== members.length) {
      findings.push(
        `${set} has ${String(allowingNone)} of ${String(members.length)} members declaring exclusiveAllowsNone. ` +
          'Every member declares it or none does, or a press keeps or releases by which member was pressed.',
      );
    }
    const pressed = members.filter((member) => member.command.pressed === true).map((member) => member.command.id);
    if (allowingNone === members.length) {
      if (pressed.length > 1) {
        findings.push(
          `${set} starts with ${String(pressed.length)} members pressed (${pressed.join(', ')}). ` +
            'A set that may hold none still holds at most one.',
        );
      }
    } else if (pressed.length !== 1) {
      findings.push(
        `${set} starts with ${String(pressed.length)} members pressed (${pressed.join(', ') || 'none'}). ` +
          'Office holds exactly one, so exactly one must start pressed.',
      );
    }
  }
  return findings;
}

describe('every exclusive set starts with exactly one member pressed, or at most one where it may hold none', () => {
  const sets = exclusiveSetsIn(everyTabByApplication);

  it('finds the sets Office holds one of, or it is checking nothing', () => {
    expect(
      Object.fromEntries([...sets].map(([set, members]) => [set, members.map((member) => member.command.label)])),
    ).toEqual({
      'word.draw.write.tools': ['Select Objects', 'Lasso Select', 'Pen', 'Highlighter', 'Eraser'],
      'powerpoint.draw.write.tools': ['Select Objects', 'Lasso Select', 'Pen', 'Highlighter', 'Eraser'],
      'excel.draw.write.tools': ['Select Objects', 'Lasso Select', 'Pen', 'Highlighter', 'Eraser'],
      'word.view.document-views': ['Read Mode', 'Print Layout', 'Web Layout', 'Outline', 'Draft'],
      'word.view.page-movement': ['Vertical', 'Side to Side'],
      'powerpoint.view.presentation-views': ['Normal', 'Outline View', 'Slide Sorter', 'Notes Page', 'Reading View'],
      'powerpoint.view.colour-greyscale': ['Colour', 'Greyscale', 'Black and White'],
      'powerpoint.view.view-direction': ['Left-to-Right', 'Right-to-Left'],
      'excel.view.workbook-views': ['Normal', 'Page Break Preview', 'Page Layout'],
      'powerpoint.black-and-white.colour-mode': [
        'Automatic',
        'Greyscale',
        'Light Greyscale',
        'Inverse Greyscale',
        'Grey with White Fill',
        'Black with Greyscale Fill',
        'Black with White Fill',
        'Black',
        'White',
        "Don't Show",
      ],
      'powerpoint.greyscale.colour-mode': [
        'Automatic',
        'Greyscale',
        'Light Greyscale',
        'Inverse Greyscale',
        'Grey with White Fill',
        'Black with Greyscale Fill',
        'Black with White Fill',
        'Black',
        'White',
        "Don't Show",
      ],
      'word.background-removal.refine': ['Mark Areas to Keep', 'Mark Areas to Remove'],
      'word.table-layout.draw.tools': ['Draw Table', 'Eraser'],
      'word.table-layout.alignment.cell-alignment': [
        'Align Top Left',
        'Align Centre Left',
        'Align Bottom Left',
        'Align Top Centre',
        'Align Centre',
        'Align Bottom Centre',
        'Align Top Right',
        'Align Centre Right',
        'Align Bottom Right',
      ],
      'powerpoint.background-removal.refine': ['Mark Areas to Keep', 'Mark Areas to Remove'],
      'powerpoint.table-design.draw-borders.tools': ['Draw Table', 'Eraser'],
      'powerpoint.table-layout.alignment.horizontal': ['Align Left', 'Align Centre', 'Align Right'],
      'powerpoint.table-layout.alignment.vertical': ['Align Top', 'Centre Vertically', 'Align Bottom'],
      'excel.background-removal.refine': ['Mark Areas to Keep', 'Mark Areas to Remove'],
    });
  });

  it('lets exactly the sets with no member standing for no tool hold none', () => {
    expect(
      [...sets]
        .filter(([, members]) => members.every((member) => member.command.exclusiveAllowsNone === true))
        .map(([set]) => set),
    ).toEqual([
      'word.background-removal.refine',
      'word.table-layout.draw.tools',
      'powerpoint.background-removal.refine',
      'powerpoint.table-design.draw-borders.tools',
      'excel.background-removal.refine',
    ]);
  });

  it('starts each on the member a new document or a fresh tab is in', () => {
    const starting = Object.fromEntries(
      [...sets].map(([set, members]) => [
        set,
        members.filter((member) => member.command.pressed === true).map((member) => member.command.label),
      ]),
    );
    expect(starting).toEqual({
      'word.draw.write.tools': ['Select Objects'],
      'powerpoint.draw.write.tools': ['Select Objects'],
      'excel.draw.write.tools': ['Select Objects'],
      'word.view.document-views': ['Print Layout'],
      'word.view.page-movement': ['Vertical'],
      'powerpoint.view.presentation-views': ['Normal'],
      'powerpoint.view.colour-greyscale': ['Colour'],
      'powerpoint.view.view-direction': ['Left-to-Right'],
      'excel.view.workbook-views': ['Normal'],
      'powerpoint.black-and-white.colour-mode': ['Automatic'],
      'powerpoint.greyscale.colour-mode': ['Automatic'],
      'word.background-removal.refine': [],
      'word.table-layout.draw.tools': [],
      'word.table-layout.alignment.cell-alignment': ['Align Top Left'],
      'powerpoint.background-removal.refine': [],
      'powerpoint.table-design.draw-borders.tools': [],
      'powerpoint.table-layout.alignment.horizontal': ['Align Left'],
      'powerpoint.table-layout.alignment.vertical': ['Align Top'],
      'excel.background-removal.refine': [],
    });
  });

  it('holds every set in the census to the rule', () => {
    expect(exclusiveSetFindings(everyTabByApplication)).toEqual([]);
  });
});

describe('the exclusive set rule can reject', () => {
  const tabOf = (id: string, commands: readonly RibbonCommand[]): RibbonTabEntry => ({
    id,
    label: id,
    appearance: 'always',
    source: { kind: 'core', tab: 'TabFixture' },
    groups: [{ id: 'GroupFixture', label: 'Fixture', priority: 'standard', controls: commands.length, inScope: true, commands }],
  });
  const member = (label: string, extra: Partial<RibbonCommand> = {}): RibbonCommand => ({
    id: `word.fixture.group.${label}`,
    label,
    toggle: true,
    exclusive: 'word.fixture.set',
    ...extra,
  });

  it('accepts a set of toggles in one tab with one pressed', () => {
    expect(exclusiveSetFindings({ word: [tabOf('view', [member('a', { pressed: true }), member('b')])] })).toEqual([]);
  });

  it('refuses a set that starts with none pressed, or with two', () => {
    expect(exclusiveSetFindings({ word: [tabOf('view', [member('a'), member('b')])] })).toEqual([
      expect.stringContaining('starts with 0 members pressed (none)'),
    ]);
    expect(
      exclusiveSetFindings({ word: [tabOf('view', [member('a', { pressed: true }), member('b', { pressed: true })])] }),
    ).toEqual([expect.stringContaining('starts with 2 members pressed')]);
  });

  it('refuses a member that is not a toggle', () => {
    const findings = exclusiveSetFindings({
      word: [tabOf('view', [member('a', { pressed: true }), member('b', { toggle: false })])],
    });
    expect(findings).toEqual([expect.stringContaining('is not a toggle')]);
  });

  it('refuses a set split across two tabs', () => {
    const findings = exclusiveSetFindings({
      word: [tabOf('view', [member('a', { pressed: true })]), tabOf('draw', [member('b')])],
    });
    expect(findings).toEqual([expect.stringContaining('has members in word draw and word view')]);
  });

  it('refuses a set of one', () => {
    expect(exclusiveSetFindings({ word: [tabOf('view', [member('a', { pressed: true })])] })).toEqual([
      expect.stringContaining('has one member'),
    ]);
  });

  it('accepts a set that may hold none starting with none or one pressed, and refuses two', () => {
    const none = { exclusiveAllowsNone: true } as const;
    expect(exclusiveSetFindings({ word: [tabOf('view', [member('a', none), member('b', none)])] })).toEqual([]);
    expect(
      exclusiveSetFindings({ word: [tabOf('view', [member('a', { ...none, pressed: true }), member('b', none)])] }),
    ).toEqual([]);
    expect(
      exclusiveSetFindings({
        word: [tabOf('view', [member('a', { ...none, pressed: true }), member('b', { ...none, pressed: true })])],
      }),
    ).toEqual([expect.stringContaining('still holds at most one')]);
  });

  it('refuses a set whose members disagree on holding none', () => {
    const findings = exclusiveSetFindings({
      word: [tabOf('view', [member('a', { pressed: true, exclusiveAllowsNone: true }), member('b')])],
    });
    expect(findings).toEqual([expect.stringContaining('1 of 2 members declaring exclusiveAllowsNone')]);
  });

  it('refuses exclusiveAllowsNone on a command in no set', () => {
    const findings = exclusiveSetFindings({
      word: [tabOf('view', [member('a', { pressed: true }), member('b'), { id: 'word.fixture.group.c', label: 'c', toggle: true, exclusiveAllowsNone: true }])],
    });
    expect(findings).toEqual([expect.stringContaining('word.fixture.group.c declares exclusiveAllowsNone and is in no set')]);
  });
});

/**
 * **What is wrong with a host's bindings over members of an exclusive set.**
 *
 * `renderCommand` writes `exclusive` onto the generic toggle, but a host override is markup the host wrote,
 * so the attribute is written by hand there, and one forgotten attribute is a member that silently stays
 * pressed beside the one somebody picked. So a binding over a member must carry the census's set, and must
 * be an element that holds a position: `<mjx-toggle-button>`, or `<mjx-split-button toggle>`. A binding
 * that claims a set the census does not declare is refused too, so the two spellings cannot drift apart.
 */
export function exclusiveBindingFindings(
  file: { readonly name: string; readonly source: string },
  commands: readonly RibbonCommand[],
): string[] {
  const byId = new Map(commands.map((command) => [command.id, command]));
  const findings: string[] = [];
  for (const bound of boundControlsIn(file.source)) {
    const command = byId.get(bound.key);
    if (command === undefined) continue;
    const declared = /(?:^|\s)exclusive="([^"]*)"/.exec(bound.attributes)?.[1];
    const allowsNone = /(?:^|\s)exclusive-allows-none(?:[\s=]|$)/.test(bound.attributes);
    if (allowsNone !== (command.exclusiveAllowsNone === true)) {
      findings.push(
        allowsNone
          ? `${file.name}: ${bound.key} is bound with exclusive-allows-none, and the census does not declare it.`
          : `${file.name}: ${bound.key} declares exclusiveAllowsNone in the census and its binding carries no ` +
              '`exclusive-allows-none`, so pressing it while it holds keeps it.',
      );
    }
    if (command.exclusive === undefined) {
      if (declared !== undefined) {
        findings.push(
          `${file.name}: ${bound.key} is bound with exclusive="${declared}", and the census declares it in no set.`,
        );
      }
      continue;
    }
    if (declared === undefined) {
      findings.push(
        `${file.name}: ${bound.key} is in ${command.exclusive} in the census and its binding carries no ` +
          '`exclusive`, so pressing another member leaves it pressed.',
      );
    } else if (declared !== command.exclusive) {
      findings.push(
        `${file.name}: ${bound.key} is bound with exclusive="${declared}", and the census puts it in ${command.exclusive}.`,
      );
    }
    const holdsPosition =
      bound.tag === 'mjx-toggle-button' ||
      (bound.tag === 'mjx-split-button' && /(?:^|\s)toggle(?:\s|$)/.test(bound.attributes));
    if (!holdsPosition) {
      findings.push(
        `${file.name}: ${bound.key} is in ${command.exclusive} and is bound as a <${bound.tag}> that holds no ` +
          'position, so nothing can release it.',
      );
    }
  }
  return findings;
}

describe('every host binding over a member of a set carries the set', () => {
  const commands = everyRibbonCommand();

  it('finds bound members at all, or it is checking nothing', () => {
    const bound = assemblySources
      .flatMap((file) => boundControlsIn(file.source))
      .filter((control) => /(?:^|\s)exclusive="/.test(control.attributes));
    // Word's and PowerPoint's split Eraser, in two hosts each. Every other member is the generic toggle.
    expect(bound.map((control) => control.key).sort()).toEqual(
      ['powerpoint.draw.write.eraser', 'powerpoint.draw.write.eraser', 'word.draw.write.eraser', 'word.draw.write.eraser'].sort(),
    );
  });

  for (const file of assemblySources) {
    it(`${file.name} binds every member with its census set`, () => {
      expect(exclusiveBindingFindings(file, commands)).toEqual([]);
    });
  }
});

describe('the exclusive binding rule can reject', () => {
  const eraser: RibbonCommand = {
    id: 'word.draw.write.eraser',
    label: 'Eraser',
    toggle: true,
    exclusive: 'word.draw.write.tools',
  };
  const loose: RibbonCommand = { id: 'word.review.ink.hide-ink', label: 'Hide Ink', toggle: true };
  const host = (key: string, tag: string, attributes: string): { name: string; source: string } => ({
    name: 'fixture.stories.ts',
    source: `const bindings = {\n  '${key}': html\`<${tag}\n    ${attributes}\n    label="x"\n  ></${tag}>\`,\n};\n`,
  });

  it('accepts a split toggle and a toggle button carrying the census set', () => {
    expect(exclusiveBindingFindings(host(eraser.id, 'mjx-split-button', 'toggle\n    exclusive="word.draw.write.tools"'), [eraser])).toEqual([]);
    expect(exclusiveBindingFindings(host(eraser.id, 'mjx-toggle-button', 'exclusive="word.draw.write.tools"'), [eraser])).toEqual([]);
    expect(exclusiveBindingFindings(host(loose.id, 'mjx-split-button', 'toggle'), [loose])).toEqual([]);
  });

  it('refuses a member bound without its set', () => {
    expect(exclusiveBindingFindings(host(eraser.id, 'mjx-split-button', 'toggle'), [eraser])).toEqual([
      expect.stringContaining('carries no `exclusive`'),
    ]);
  });

  it('refuses a binding naming a different set, or a set the census does not declare', () => {
    expect(
      exclusiveBindingFindings(host(eraser.id, 'mjx-split-button', 'toggle\n    exclusive="word.draw.pens"'), [eraser]),
    ).toEqual([expect.stringContaining('the census puts it in word.draw.write.tools')]);
    expect(
      exclusiveBindingFindings(host(loose.id, 'mjx-split-button', 'toggle\n    exclusive="word.review.ink"'), [loose]),
    ).toEqual([expect.stringContaining('declares it in no set')]);
  });

  it('holds exclusive-allows-none on a binding to the census, in both directions', () => {
    const pencil: RibbonCommand = {
      id: 'word.background-removal.refine.mark-areas-to-keep',
      label: 'Mark Areas to Keep',
      toggle: true,
      exclusive: 'word.background-removal.refine',
      exclusiveAllowsNone: true,
    };
    const withSet = 'exclusive="word.background-removal.refine"';
    expect(
      exclusiveBindingFindings(host(pencil.id, 'mjx-toggle-button', `${withSet}\n    exclusive-allows-none`), [pencil]),
    ).toEqual([]);
    expect(exclusiveBindingFindings(host(pencil.id, 'mjx-toggle-button', withSet), [pencil])).toEqual([
      expect.stringContaining('carries no `exclusive-allows-none`'),
    ]);
    expect(
      exclusiveBindingFindings(
        host(eraser.id, 'mjx-split-button', 'toggle\n    exclusive="word.draw.write.tools"\n    exclusive-allows-none'),
        [eraser],
      ),
    ).toEqual([expect.stringContaining('the census does not declare it')]);
  });

  it('refuses a member bound as something with no position', () => {
    expect(
      exclusiveBindingFindings(host(eraser.id, 'mjx-split-button', 'exclusive="word.draw.write.tools"'), [eraser]),
    ).toEqual([expect.stringContaining('holds no position')]);
    expect(exclusiveBindingFindings(host(eraser.id, 'mjx-button', 'exclusive="word.draw.write.tools"'), [eraser])).toEqual([
      expect.stringContaining('holds no position'),
    ]);
  });
});

// ── the collapse ceiling ─────────────────────────────────────────────────────

/**
 * What is wrong with one group's declared survivors.
 *
 * ⚠ **Counts `essential` alone.** Until unit 2b `essentialCommands()` also counted every toggle, so
 * this ceiling was secretly a ceiling on *state commands* and refused a fourth alignment. A state is
 * not a survivor; this is now the rule `demotionRules` states and nothing more.
 *
 * What it can check of rule 1 — *nothing that opens anything* — is the part the data can see: an
 * essential command must be drawn by the generic button or toggle, never by a host's override,
 * because every override in this catalogue exists precisely because the command is richer than a
 * button (a split button, a picker, a gallery, a field). Whether **Office** draws a generic-looking
 * command as a split button is a judgement recorded beside each group in `dev/ribbons/census.ts`,
 * and it is stated as a judgement here rather than dressed up as a check.
 */
export function survivorFindings(
  where: string,
  commands: readonly RibbonCommand[],
  overridden: ReadonlySet<string>,
): string[] {
  const findings: string[] = [];
  const survivors = commands.filter((command) => command.essential === true);
  if (survivors.length > essentialCommandLimit) {
    findings.push(
      `${where} declares ${String(survivors.length)} essential commands. A survivor row longer ` +
        `than ${String(essentialCommandLimit)} is a ribbon again.`,
    );
  }
  if (survivors.length > 0 && survivors.length === commands.length) {
    findings.push(
      `${where} declares every one of its commands essential, so its collapsed popup opens empty.`,
    );
  }
  for (const command of survivors) {
    if (command.icon === undefined) {
      findings.push(`${command.id} is essential and has no icon; a survivor row has no room for a label.`);
    }
    if (overridden.has(command.id)) {
      findings.push(
        `${command.id} is essential and a host binds its own control over it. Every override is ` +
          'richer than a button — demotion rule 1 refuses it.',
      );
    }
  }
  return findings;
}

describe('the demotion ceiling', () => {
  const authored = ribbonApplicationNames.flatMap((application) =>
    everyRibbonTab(application).flatMap((tab) =>
      tab.groups
        .filter((group) => group.commands !== undefined)
        .map((group) => ({ where: `${application}/${tab.id}/${group.label}`, group })),
    ),
  );

  it('sweeps authored groups, some of which declare survivors', () => {
    expect(authored.length).toBeGreaterThan(20);
    expect(
      authored.filter(({ group }) => essentialCommands(group).length > 0).length,
      'no authored group declares a survivor, so every assertion below is about an empty set',
    ).toBeGreaterThan(5);
  });

  it('keeps every authored group inside it, counting declared survivors only', () => {
    const findings = authored.flatMap(({ where, group }) =>
      survivorFindings(where, group.commands ?? [], overriddenIds),
    );
    expect(findings).toEqual([]);
  });

  it('no longer counts a toggle as a survivor', () => {
    // The unit's own defect, pinned: Word's Font group declares six toggles and two survivors — the
    // survivors named rather than counted, so a ceiling-sized coincidence cannot pass for the rule.
    const font = ribbonCensus.word
      .find((tab) => tab.id === 'home')
      ?.groups.find((group) => group.id === 'GroupFont');
    expect(font).toBeDefined();
    const toggles = (font?.commands ?? []).filter((command) => command.toggle === true);
    expect(toggles.length, 'Word Font has fewer toggles than the ceiling').toBeGreaterThan(
      essentialCommandLimit,
    );
    expect(
      essentialCommands(font ?? { id: '', label: '', priority: 'primary', controls: 0, inScope: true }).map(
        (command) => command.id,
      ),
    ).toEqual(['word.home.font.bold', 'word.home.font.italic']);
  });

  it('refuses a fourth survivor, a survivor with no icon, an overridden survivor, and an empty popup', () => {
    const command = (id: string, over: Partial<RibbonCommand> = {}): RibbonCommand => ({
      id: `word.home.specimen.${id}`,
      label: id,
      icon: 'text-bold',
      ...over,
    });
    expect(
      survivorFindings('fine', [command('a', { essential: true }), command('b')], new Set()),
    ).toEqual([]);
    expect(
      survivorFindings(
        'four',
        ['a', 'b', 'c', 'd'].map((id) => command(id, { essential: true })).concat(command('e')),
        new Set(),
      ),
    ).toHaveLength(1);
    expect(
      survivorFindings(
        'no icon',
        [{ id: 'word.home.specimen.a', label: 'a', essential: true }, command('b')],
        new Set(),
      )[0],
    ).toContain('no icon');
    expect(
      survivorFindings('bound', [command('a', { essential: true }), command('b')], new Set(['word.home.specimen.a']))[0],
    ).toContain('rule 1');
    expect(survivorFindings('empty', [command('a', { essential: true })], new Set())[0]).toContain(
      'opens empty',
    );
  });
});

// ── where a survivor draws ───────────────────────────────────────────────────

/** How a group presents its commands, as a function a gate can swap out. */
type Presenter = (
  presentation: GroupPresentation,
  commands: readonly RibbonCommand[],
  isEssential: (command: RibbonCommand) => boolean,
) => readonly RibbonCommand[];

const declaredEssential = (command: RibbonCommand): boolean => command.essential === true;

/**
 * **The order a group presents its commands in, against the order it declares them** — for every
 * presentation.
 *
 * The expectation is written out per presentation rather than derived from `groupPresentations`,
 * so a presenter that consulted the wrong field would disagree with it: `full` and `reduced` present
 * exactly the declaration, and `collapsed` presents the declared survivors, then the rest, each in
 * declared order.
 *
 * The presenter is a parameter so the instrument tests can hand it the two rules this unit exists
 * to refuse — survivors first (the old row) and survivors last (`d01cf93`) — and watch both fail on
 * the census's own data. `presentedCommandOrder` is the default because it is what
 * `<mjx-ribbon-group>` executes; `tests/browser/ribbon.spec.ts` reads the real slots.
 */
export function orderFindings(
  where: string,
  commands: readonly RibbonCommand[],
  present: Presenter = presentedCommandOrder,
): string[] {
  const survivors = commands.filter(declaredEssential);
  const rest = commands.filter((command) => !declaredEssential(command));
  const expected: Readonly<Record<GroupPresentation, readonly RibbonCommand[]>> = {
    full: commands,
    reduced: commands,
    collapsed: [...survivors, ...rest],
  };
  const ids = (list: readonly RibbonCommand[]): string => list.map((command) => command.id).join(', ');
  return groupPresentationOrder.flatMap((presentation) => {
    const actual = ids(present(presentation, commands, declaredEssential));
    const wanted = ids(expected[presentation]);
    return actual === wanted
      ? []
      : [`${where} presents [${actual}] when ${presentation}, and the rule says [${wanted}].`];
  });
}

/** Whether a group puts a survivor with a non-survivor on each side — the shape both blanket rules get wrong. */
function interleavesSurvivors(commands: readonly RibbonCommand[]): boolean {
  // ES2022 has no `findLastIndex`, and this workspace's `lib` is ES2022.
  const survivorIndices = commands.flatMap((command, index) => (declaredEssential(command) ? [index] : []));
  const first = survivorIndices[0];
  const last = survivorIndices[survivorIndices.length - 1];
  if (first === undefined || last === undefined) return false;
  return (
    commands.slice(0, first).some((command) => !declaredEssential(command)) &&
    commands.slice(last + 1).some((command) => !declaredEssential(command))
  );
}

const survivorsFirst: Presenter = (_presentation, commands, isEssential) => [
  ...commands.filter(isEssential),
  ...commands.filter((command) => !isEssential(command)),
];

const survivorsLast: Presenter = (_presentation, commands, isEssential) => [
  ...commands.filter((command) => !isEssential(command)),
  ...commands.filter(isEssential),
];

describe('every authored group presents its commands in the order it declares them', () => {
  const authored = ribbonApplicationNames.flatMap((application) =>
    everyRibbonTab(application).flatMap((tab) =>
      tab.groups
        .filter((group) => group.commands !== undefined)
        .map((group) => ({ where: `${application}/${tab.id}/${group.label}`, commands: group.commands ?? [] })),
    ),
  );

  it('in all three presentations', () => {
    expect(authored.flatMap(({ where, commands }) => orderFindings(where, commands))).toEqual([]);
  });

  it('on data that is DISCRIMINATING — a survivor with a command either side of it', () => {
    // Without this the two refusals below could pass over groups whose survivors happen to be
    // declared first or last, where a blanket rule and the declared order coincide.
    expect(
      authored.filter(({ commands }) => interleavesSurvivors(commands)).map(({ where }) => where),
    ).toContain('word/home/Font');
  });
});

describe('the order rule can reject', () => {
  const authored = ribbonApplicationNames.flatMap((application) =>
    everyRibbonTab(application).flatMap((tab) =>
      tab.groups.map((group) => ({ where: `${application}/${tab.id}/${group.label}`, commands: group.commands ?? [] })),
    ),
  );

  it('refuses survivors drawn first — the row unit 2 shipped', () => {
    const findings = authored.flatMap(({ where, commands }) => orderFindings(where, commands, survivorsFirst));
    expect(findings.some((finding) => finding.startsWith('word/home/Font presents') && finding.includes('when full'))).toBe(true);
  });

  it('refuses survivors drawn last — the row d01cf93 tried and reverted', () => {
    const findings = authored.flatMap(({ where, commands }) => orderFindings(where, commands, survivorsLast));
    expect(findings.some((finding) => finding.startsWith('word/home/Font presents') && finding.includes('when full'))).toBe(true);
  });

  it('refuses a presenter that forgets to move the survivors out when collapsed', () => {
    const neverSplits: Presenter = (_presentation, commands) => commands;
    const findings = authored.flatMap(({ where, commands }) => orderFindings(where, commands, neverSplits));
    expect(findings.some((finding) => finding.includes('when collapsed'))).toBe(true);
    expect(findings.some((finding) => finding.includes('when full'))).toBe(false);
  });

  it('reads interleaving correctly on hand-made groups', () => {
    const c = (id: string, essential = false): RibbonCommand => ({ id: `word.home.x.${id}`, label: id, essential });
    expect(interleavesSurvivors([c('a'), c('b', true), c('c')])).toBe(true);
    expect(interleavesSurvivors([c('a', true), c('b')])).toBe(false);
    expect(interleavesSurvivors([c('a'), c('b', true)])).toBe(false);
    expect(interleavesSurvivors([c('a'), c('b')])).toBe(false);
  });
});

// ── every surface a binding opens exists ─────────────────────────────────────

/**
 * Every command id a `commandMenu(host, '<id>', …)` call declares a menu for.
 *
 * `stories/ribbons/ribbon-parts.ts`'s `commandMenu` documents that its first argument is always
 * spelt `host` at a call site; this is the reader that relies on it.
 */
export function commandMenusIn(source: string): string[] {
  return [...source.matchAll(/commandMenu\(\s*host\s*,\s*'([a-z]+(?:\.[a-z0-9-]+){3})'/g)].map(
    (match) => match[1] ?? '',
  );
}

/** Every `data-opens` in a host's source, attributed to the binding key written nearest above it. */
export function dataOpensIn(source: string): { readonly key: string | undefined; readonly opens: string }[] {
  const keys = [...source.matchAll(/'([a-z]+(?:\.[a-z0-9-]+){3})'\s*:/g)].map((match) => ({
    key: match[1] ?? '',
    at: match.index,
  }));
  return [...source.matchAll(/\bdata-opens="([^"]+)"/g)].map((match) => ({
    key: keys.filter((entry) => entry.at < match.index).at(-1)?.key,
    opens: match[1] ?? '',
  }));
}

/**
 * **Whether a host draws the `appearance: 'view'` tabs.** `Ribbons/*` passes `includeViewTabs: true` to
 * `tabsFor`, so a reviewer can audit a tab Office shows only inside its view; `Shell/*` draws the strip Office
 * shows, and never does. `every surface a binding opens exists` holds this table to the hosts' own source.
 */
export const hostDrawsViewTabs: Readonly<Record<RibbonSurfaceHost, boolean>> = { ribbons: true, shell: false };

/**
 * **Which contextual sets each host draws, per application**: every built set, or the named ones.
 *
 * `Ribbons/*` calls `<app>ContextualSets` with no `sets`, so every contextual tab has a story. `Shell/*` names the one
 * set its document's selection would show, because Office never shows four at once: Table Tools in Word and Excel,
 * Picture Tools in PowerPoint. A contextual command's menu is therefore required only of the hosts that draw its set,
 * exactly as a view tab's is required only of the hosts that draw view tabs, and a host that opens the menu of a set
 * it never draws has written a binding to nothing. `every surface a binding opens exists` holds this table to the
 * hosts' own source.
 */
export const hostContextualSets: Readonly<
  Record<RibbonSurfaceHost, Readonly<Record<RibbonApplication, 'every' | readonly string[]>>>
> = {
  ribbons: { word: 'every', powerpoint: 'every', excel: 'every' },
  shell: { word: ['table-tools'], powerpoint: ['picture-tools'], excel: ['table-tools'] },
};

/** The contextual set holding the tab a command id names, or `undefined` for a command on a core, File or view tab. */
export function contextualSetOfCommand(command: string): string | undefined {
  const [application, tabId] = command.split('.');
  const known = ribbonApplicationNames.find((name) => name === application);
  if (known === undefined) return undefined;
  return ribbonContextualSets[known].find((set) => set.tabs.some((tab) => tab.id === tabId))?.id;
}

/**
 * **Whether a host draws the tab a command lives on**: a view tab only where `hostDrawsViewTabs` says, a contextual
 * tab only where `hostContextualSets` names its set, and every other tab everywhere.
 */
export function hostDrawsTabOf(host: RibbonSurfaceHost, application: RibbonApplication, command: string): boolean {
  const appearance = appearanceOfCommand(command);
  if (appearance === 'view') return hostDrawsViewTabs[host];
  if (appearance !== 'contextual') return true;
  const drawn = hostContextualSets[host][application];
  const set = contextualSetOfCommand(command);
  return drawn === 'every' || (set !== undefined && drawn.includes(set));
}

/**
 * **The contextual sets an assembly source draws**, read from its `<app>ContextualSets(…)` calls: the ids of a literal
 * `sets: [...]` written as the call's first option, or `every` for a call that names none. `undefined` for a source
 * with no call at all.
 */
export function contextualSetsDrawnIn(source: string): 'every' | readonly string[] | undefined {
  const calls = [...source.matchAll(/\b[a-z][A-Za-z]*ContextualSets\(\s*(?:\{\s*sets:\s*\[([^\]]*)\])?/g)];
  if (calls.length === 0) return undefined;
  if (calls.some((call) => call[1] === undefined)) return 'every';
  return [...new Set(calls.flatMap((call) => [...(call[1] ?? '').matchAll(/'([a-z][a-z0-9-]*)'/g)].map((id) => id[1] ?? '')))];
}

/** The appearance of the tab a command id names, or `always` for an id the census does not declare. */
export function appearanceOfCommand(command: string): TabAppearance {
  const [application, tabId] = command.split('.');
  const known = ribbonApplicationNames.find((name) => name === application);
  if (known === undefined) return 'always';
  return everyRibbonTab(known).find((entry) => entry.id === tabId)?.appearance ?? 'always';
}

/** One host's assembly source, and which application and host it is. */
interface HostSource {
  readonly name: string;
  readonly application: RibbonApplication;
  readonly host: RibbonSurfaceHost;
  readonly source: string;
}

/**
 * **What is wrong with the surfaces the hosts open.**
 *
 * `openDeclaredSurface` looks the `data-opens` id up and returns silently when nothing has it — a
 * binding with a misspelt id is a button that looks right and opens nothing, which no screenshot
 * shows. And the menus a unit writes live in a different file from the bindings that open them
 * (`stories/ribbons/insert-menus.ts` against six host files), so the two can drift apart on either
 * side. Four refusals:
 *
 * 1. a `data-opens` that names neither a literal `id="…"` in its own file nor a declared menu;
 * 2. a binding that opens a declared menu belonging to a **different** command — the copy-and-paste
 *    slip that opens Footer's menu from Header;
 * 3. a declared menu that a host of its application **which draws the command's tab** never opens from
 *    that command's own binding — a view tab's menu is required of `Ribbons/*` alone, because `Shell/*`
 *    never draws the tab (`hostDrawsViewTabs`), and a contextual tab's menu only of the hosts that draw its
 *    set (`hostContextualSets`);
 * 4. a host that binds declared menus and never renders them;
 * 5. a host that opens the menu of a tab it never draws, which is a binding to nothing.
 */
export function surfaceFindings(
  hosts: readonly HostSource[],
  menuSources: readonly string[],
): string[] {
  const menus = [...new Set(menuSources.flatMap(commandMenusIn))];
  const findings: string[] = [];
  for (const file of hosts) {
    const literalIds = new Set([...file.source.matchAll(/\bid="([^"]+)"/g)].map((match) => match[1] ?? ''));
    const menuForId = new Map(menus.map((command) => [commandSurfaceId(file.host, command), command]));
    const opened = dataOpensIn(file.source);
    for (const { key, opens } of opened) {
      const menu = menuForId.get(opens);
      if (menu !== undefined) {
        if (!hostDrawsTabOf(file.host, file.application, menu)) {
          const tab =
            appearanceOfCommand(menu) === 'view'
              ? 'a view tab'
              : `in the ${contextualSetOfCommand(menu) ?? '(unknown)'} contextual set`;
          findings.push(
            `${file.name}: the binding for ${key ?? '(no command)'} opens the menu declared for ${menu}, ` +
              `whose tab is ${tab}, which this host never draws, so the binding is a binding to nothing.`,
          );
        }
        if (key !== menu) {
          findings.push(
            `${file.name}: the binding for ${key ?? '(no command)'} opens the menu declared for ${menu}.`,
          );
        }
      } else if (!literalIds.has(opens)) {
        findings.push(
          `${file.name}: data-opens="${opens}" (in the binding for ${key ?? '(no command)'}) names no ` +
            'element on the page, so openDeclaredSurface finds nothing and the control opens nothing.',
        );
      }
    }
    const ownMenus = menus.filter(
      (command) => command.split('.')[0] === file.application && hostDrawsTabOf(file.host, file.application, command),
    );
    for (const command of ownMenus) {
      const wanted = commandSurfaceId(file.host, command);
      if (!opened.some(({ key, opens }) => key === command && opens === wanted)) {
        findings.push(
          `${file.name} never opens the menu declared for ${command} from that command's binding ` +
            `(data-opens="${wanted}"), so the menu is unreachable on this host.`,
        );
      }
    }
    const renders = new RegExp(`\\b[a-z][A-Za-z]*Menus\\(\\s*'${file.application}'\\s*,\\s*'${file.host}'\\s*\\)`);
    if (ownMenus.length > 0 && !renders.test(file.source)) {
      findings.push(
        `${file.name} binds commands to declared menus and never renders them for its host ` +
          `('${file.application}', '${file.host}').`,
      );
    }
  }
  return findings;
}

describe('every surface a binding opens exists', () => {
  const ribbonsDirectory = resolve(import.meta.dirname, '../stories/ribbons');
  const menuSources = readdirSync(ribbonsDirectory)
    .filter((name) => name.endsWith('.ts') && !name.endsWith('.stories.ts'))
    .map((name) => readFileSync(resolve(ribbonsDirectory, name), 'utf8'));
  const hosts: HostSource[] = ribbonApplicationNames.flatMap((application) =>
    ribbonSurfaceHostNames.map((host) => ({
      name: `${host}/${application}.stories.ts`,
      application,
      host,
      source: readFileSync(resolve(import.meta.dirname, `../stories/${host}/${application}.stories.ts`), 'utf8'),
    })),
  );

  it('reads menus and bindings at all, or it is checking nothing', () => {
    const menus = new Set(menuSources.flatMap(commandMenusIn));
    expect(menus.size, 'no commandMenu(host, …) call found under stories/ribbons/').toBeGreaterThan(40);
    for (const application of ribbonApplicationNames) {
      expect([...menus].some((command) => command.startsWith(`${application}.`))).toBe(true);
    }
    expect(hosts.flatMap((file) => dataOpensIn(file.source)).length).toBeGreaterThan(100);
  });

  it('every menu declared is opened by both hosts, and every data-opens resolves to its own command', () => {
    expect(surfaceFindings(hosts, menuSources)).toEqual([]);
  });

  it('every declared menu names a command the census declares', () => {
    const declared = new Set(everyRibbonCommand().map((command) => command.id));
    expect(menuSources.flatMap(commandMenusIn).filter((command) => !declared.has(command))).toEqual([]);
  });

  it('draws view tabs in exactly the hosts `hostDrawsViewTabs` says, read from their source', () => {
    for (const file of hosts) {
      expect(/\bincludeViewTabs:\s*true\b/.test(file.source), file.name).toBe(hostDrawsViewTabs[file.host]);
    }
  });

  it('finds a view tab with declared menus, or the view-tab exemption is exempting nothing', () => {
    expect(menuSources.flatMap(commandMenusIn).some((command) => appearanceOfCommand(command) === 'view')).toBe(true);
  });

  it('draws contextual sets in exactly the hosts `hostContextualSets` says, read from their source', () => {
    for (const file of hosts) {
      expect(contextualSetsDrawnIn(file.source), file.name).toEqual(hostContextualSets[file.host][file.application]);
    }
  });

  it('names only built sets in `hostContextualSets`', () => {
    for (const host of ribbonSurfaceHostNames) {
      for (const application of ribbonApplicationNames) {
        const drawn = hostContextualSets[host][application];
        const built = ribbonContextualSets[application].map((set) => set.id);
        if (drawn !== 'every') expect(drawn.filter((id) => !built.includes(id)), `${host}/${application}`).toEqual([]);
      }
    }
  });

  it('finds a contextual menu a shell does not draw, or the contextual exemption is exempting nothing', () => {
    expect(
      menuSources
        .flatMap(commandMenusIn)
        .some((command) => {
          const application = ribbonApplicationNames.find((name) => name === command.split('.')[0]);
          return (
            application !== undefined &&
            appearanceOfCommand(command) === 'contextual' &&
            !hostDrawsTabOf('shell', application, command)
          );
        }),
    ).toBe(true);
  });
});

describe('the surface rule can reject', () => {
  const menu = "commandMenu(host, 'word.insert.text.wordart', 'WordArt', item('Fill'))";
  const binding = (host: RibbonSurfaceHost, key: string, opens: string, renders = true): HostSource => ({
    name: `${host}/word.stories.ts`,
    application: 'word',
    host,
    source:
      `const bindings = {\n  '${key}': html\`<mjx-button data-opens="${opens}"></mjx-button>\`,\n};\n` +
      (renders ? `html\`\${insertMenus('word', '${host}')}\`` : ''),
  });

  it('accepts a menu both hosts open from its own binding', () => {
    expect(
      surfaceFindings(
        [
          binding('ribbons', 'word.insert.text.wordart', 'ribbons-word-insert-text-wordart'),
          binding('shell', 'word.insert.text.wordart', 'shell-word-insert-text-wordart'),
        ],
        [menu],
      ),
    ).toEqual([]);
  });

  it('reads a binding key and its data-opens together, and a menu call by its literal id', () => {
    expect(commandMenusIn(menu)).toEqual(['word.insert.text.wordart']);
    expect(commandMenusIn("commandMenu(host, commandId, 'Computed')"), 'a computed id is invisible').toEqual([]);
    expect(dataOpensIn(binding('shell', 'word.insert.text.wordart', 'x').source)).toEqual([
      { key: 'word.insert.text.wordart', opens: 'x' },
    ]);
  });

  it('refuses a data-opens that names nothing', () => {
    const findings = surfaceFindings([binding('ribbons', 'word.home.font.name', 'ribbons-nowhere')], []);
    expect(findings).toHaveLength(1);
    expect(findings[0]).toContain('names no element');
  });

  it('accepts a data-opens that names a literal id in its own file', () => {
    const file = binding('ribbons', 'word.home.clipboard.paste', 'ribbons-word-paste');
    expect(
      surfaceFindings([{ ...file, source: `${file.source}\n<mjx-menu id="ribbons-word-paste"></mjx-menu>` }], []),
    ).toEqual([]);
  });

  it("refuses a binding that opens a sibling's menu, and the menu its own command then never gets", () => {
    const findings = surfaceFindings(
      [binding('ribbons', 'word.insert.text.text-box', 'ribbons-word-insert-text-wordart')],
      [menu],
    );
    expect(findings.some((finding) => finding.includes('opens the menu declared for word.insert.text.wordart'))).toBe(true);
    expect(findings.some((finding) => finding.includes('never opens the menu'))).toBe(true);
  });

  it("requires a view tab's menu of the host that draws view tabs, and of no other", () => {
    const viewMenu = "commandMenu(host, 'word.print-preview.page-setup.margins', 'Margins', item('Normal'))";
    expect(appearanceOfCommand('word.print-preview.page-setup.margins')).toBe('view');
    expect(
      surfaceFindings(
        [
          binding('ribbons', 'word.print-preview.page-setup.margins', 'ribbons-word-print-preview-page-setup-margins'),
          binding('shell', 'word.insert.text.text-box', 'shell-word-paste'),
        ],
        [viewMenu],
      ).filter((finding) => !finding.includes('names no element')),
    ).toEqual([]);
    expect(
      surfaceFindings([binding('ribbons', 'word.insert.text.text-box', 'ribbons-word-paste')], [viewMenu]).some(
        (finding) => finding.includes('never opens the menu declared for word.print-preview.page-setup.margins'),
      ),
    ).toBe(true);
  });

  it("refuses a shell that opens a view tab's menu", () => {
    const viewMenu = "commandMenu(host, 'word.print-preview.page-setup.margins', 'Margins', item('Normal'))";
    const findings = surfaceFindings(
      [binding('shell', 'word.print-preview.page-setup.margins', 'shell-word-print-preview-page-setup-margins')],
      [viewMenu],
    );
    expect(findings).toHaveLength(1);
    expect(findings[0]).toContain('a binding to nothing');
  });

  it('reads the contextual sets a host draws from its calls', () => {
    expect(contextualSetsDrawnIn("${powerpointContextualSets({ sets: ['picture-tools'] })}")).toEqual(['picture-tools']);
    expect(contextualSetsDrawnIn("${wordContextualSets({\n  sets: ['table-tools'],\n  controls: {} })}")).toEqual([
      'table-tools',
    ]);
    expect(contextualSetsDrawnIn('${excelContextualSets({ controls: bindings })}')).toBe('every');
    expect(contextualSetsDrawnIn('${wordContextualSets()}')).toBe('every');
    expect(contextualSetsDrawnIn('${wordTabs()}')).toBeUndefined();
  });

  it("requires a contextual menu of the hosts that draw its set, and of no other", () => {
    const tableMenu =
      "commandMenu(host, 'powerpoint.table-design.table-styles.borders', 'Borders', item('No Border'))";
    const pptBinding = (host: RibbonSurfaceHost, key: string, opens: string): HostSource => ({
      name: `${host}/powerpoint.stories.ts`,
      application: 'powerpoint',
      host,
      source:
        `const bindings = {\n  '${key}': html\`<mjx-button data-opens="${opens}"></mjx-button>\`,\n};\n` +
        `html\`\${tableToolsMenus('powerpoint', '${host}')}\``,
    });
    expect(appearanceOfCommand('powerpoint.table-design.table-styles.borders')).toBe('contextual');
    expect(contextualSetOfCommand('powerpoint.table-design.table-styles.borders')).toBe('table-tools');
    expect(hostDrawsTabOf('shell', 'powerpoint', 'powerpoint.table-design.table-styles.borders')).toBe(false);
    expect(hostDrawsTabOf('shell', 'word', 'word.table-design.borders.borders')).toBe(true);
    expect(
      surfaceFindings(
        [
          pptBinding(
            'ribbons',
            'powerpoint.table-design.table-styles.borders',
            'ribbons-powerpoint-table-design-table-styles-borders',
          ),
          pptBinding('shell', 'powerpoint.insert.text.text-box', 'shell-ppt-paste'),
        ],
        [tableMenu],
      ).filter((finding) => !finding.includes('names no element')),
    ).toEqual([]);
    expect(
      surfaceFindings([pptBinding('ribbons', 'powerpoint.insert.text.text-box', 'ribbons-ppt-paste')], [tableMenu]).some(
        (finding) => finding.includes('never opens the menu declared for powerpoint.table-design.table-styles.borders'),
      ),
    ).toBe(true);
  });

  it('refuses a shell that opens the menu of a contextual set it never draws', () => {
    const tableMenu =
      "commandMenu(host, 'powerpoint.table-design.table-styles.borders', 'Borders', item('No Border'))";
    const findings = surfaceFindings(
      [
        {
          name: 'shell/powerpoint.stories.ts',
          application: 'powerpoint',
          host: 'shell',
          source:
            "const controls = {\n  'powerpoint.table-design.table-styles.borders': html`<mjx-split-button " +
            'data-opens="shell-powerpoint-table-design-table-styles-borders"></mjx-split-button>`,\n};\n' +
            "html`${tableToolsMenus('powerpoint', 'shell')}`",
        },
      ],
      [tableMenu],
    );
    expect(findings).toHaveLength(1);
    expect(findings[0]).toContain('in the table-tools contextual set');
    expect(findings[0]).toContain('a binding to nothing');
  });

  it('refuses a menu one host never opens, and a host that never renders its menus', () => {
    expect(surfaceFindings([binding('shell', 'word.insert.text.text-box', 'shell-word-paste')], [menu]).some(
      (finding) => finding.includes('never opens the menu declared for word.insert.text.wordart'),
    )).toBe(true);
    const unrendered = surfaceFindings(
      [binding('shell', 'word.insert.text.wordart', 'shell-word-insert-text-wordart', false)],
      [menu],
    );
    expect(unrendered).toHaveLength(1);
    expect(unrendered[0]).toContain('never renders them');
  });
});

// ── the survivor slot is the census's to claim ───────────────────────────────

/** Every place an assembly source writes a literal `slot="essential"`. */
function essentialClaimsIn(source: string): number {
  return [...source.matchAll(/\bslot\s*=\s*["']essential["']/g)].length;
}

describe('no host claims the survivor slot', () => {
  it('reads a claim the way markup writes one, and nothing else', () => {
    expect(essentialClaimsIn('<mjx-split-button\n  slot="essential"')).toBe(1);
    expect(essentialClaimsIn("<x slot='essential'>")).toBe(1);
    expect(essentialClaimsIn('<x slot="dialog-launcher">')).toBe(0);
    expect(essentialClaimsIn("slot=${command.essential === true ? 'essential' : nothing}")).toBe(0);
  });

  for (const file of assemblySources) {
    it(`${file.name} leaves survivors to the census`, () => {
      expect(
        essentialClaimsIn(file.source),
        `${file.name} writes slot="essential" on an override. Survivors are declared in ` +
          'dev/ribbons/census.ts, and every override is a control richer than a button — the ' +
          'Paste split buttons claimed the slot until unit 2b, in all six of these files.',
      ).toBe(0);
    });
  }
});
