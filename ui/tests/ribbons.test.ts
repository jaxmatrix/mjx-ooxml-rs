import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

import {
  essentialCommands,
  everyRibbonCommand,
  ribbonApplicationNames,
  ribbonCensus,
  ribbonCensusSource,
  strongestPriority,
  tabAppearanceNames,
  type RibbonApplication,
  type RibbonCommand,
  type RibbonGroupEntry,
  type RibbonTabEntry,
} from '../dev/ribbons/census.ts';
import { controlSizes, type ControlSize } from '../src/controls/control-states.ts';
import { iconRequests } from '../src/icons/manifest.ts';
import { essentialCommandLimit, groupPriorityNames } from '../src/ribbon/ribbon-model.ts';

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

/** `GroupFont 43` strings for one tab of one application, sorted. */
function expectedGroups(application: RibbonApplication, tab: RibbonTabEntry): string[] {
  const app = ribbonCensusSource.app[application];
  const source = tab.source;
  if (source.kind === 'core') {
    return rows
      .filter(
        (row) =>
          row.app === app &&
          row.tabSet === ribbonCensusSource.coreTabSet &&
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

describe('the ribbon census', () => {
  it('has rows at all, which is the assertion every other one here rests on', () => {
    expect(rows.length, `${ribbonCensusSource.file} parsed to nothing`).toBeGreaterThan(1000);
    expect(
      rows.some((row) => row.tabSet === ribbonCensusSource.coreTabSet),
      'no core-tab rows, so every comparison below would be empty against empty',
    ).toBe(true);
  });

  for (const application of ribbonApplicationNames) {
    for (const tab of ribbonCensus[application]) {
      it(`${application}/${tab.id} names exactly the groups the census marks in scope`, () => {
        const expected = expectedGroups(application, tab);
        expect(
          expected.length,
          `the census has no in-scope rows for ${application}/${tab.id}, which cannot be right`,
        ).toBeGreaterThan(0);

        const declared = tab.groups
          .map((group) => `${group.id} ${String(group.controls)}`)
          .sort();

        expect(
          declared,
          `dev/ribbons/census.ts has drifted from ${ribbonCensusSource.file} for ` +
            `${application}/${tab.id}. The declaration is a transcription and the TSV is the ` +
            'source; a ribbon whose groups are no longer Office’s groups is a ribbon somebody ' +
            'invented.',
        ).toEqual(expected);
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
      ribbonCensus[application].flatMap((tab) =>
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
      const ids = ribbonCensus[application].map((tab) => tab.id);
      expect(new Set(ids).size, `${application} declares a tab id twice`).toBe(ids.length);
      for (const tab of ribbonCensus[application]) {
        expect(tab.id, `${application}/${tab.id} is not kebab case`).toMatch(/^[a-z][a-z0-9-]*$/);
        expect(tab.label.trim(), `${application}/${tab.id} has no label`).not.toBe('');
        expect(tabAppearanceNames).toContain(tab.appearance);
        expect(tab.groups.length, `${application}/${tab.id} has no groups`).toBeGreaterThan(0);
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
      const declared = ribbonCensus[application]
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
      ribbonCensus[application].flatMap((tab) =>
        rubricFindings(tab).map((finding) => `${application}/${finding}`),
      ),
    );
    expect(findings).toEqual([]);
  });

  it('uses only priorities the ribbon model declares', () => {
    for (const application of ribbonApplicationNames) {
      for (const tab of ribbonCensus[application]) {
        for (const group of tab.groups) {
          expect(groupPriorityNames).toContain(group.priority);
        }
      }
    }
  });

  it('gives every tab at least one primary or standard group, so a whole tab cannot give way at once', () => {
    for (const application of ribbonApplicationNames) {
      for (const tab of ribbonCensus[application]) {
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
      for (const tab of ribbonCensus[application]) {
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
  if (command.icon === undefined) return [];
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
  });

  it('catches an icon the manifest never requested', () => {
    const findings = iconFindings({ id: 'word.home.font.x', label: 'X', icon: 'not-an-icon' });
    expect(findings).toHaveLength(1);
    expect(findings[0]).toContain('does not request at all');
  });

  it('catches a large button whose icon has no 24px drawing', () => {
    // `table` is drawn at 20 alone, and `large` asks for 24 — the exact shape of the defect the
    // ribbon programme hit with `folder-open`, and the reason this suite exists.
    const findings = iconFindings({
      id: 'word.insert.tables.table',
      label: 'Table',
      icon: 'table',
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

describe('every override a host binds names a command that exists', () => {
  const files = ['ribbons/word', 'ribbons/powerpoint', 'ribbons/excel', 'shell/word', 'shell/powerpoint', 'shell/excel'].map(
    (name) => ({
      name: `${name}.stories.ts`,
      source: readFileSync(resolve(import.meta.dirname, `../stories/${name}.stories.ts`), 'utf8'),
    }),
  );
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

// ── the collapse ceiling ─────────────────────────────────────────────────────

describe('the demotion ceiling', () => {
  it('keeps every authored group inside it', () => {
    for (const application of ribbonApplicationNames) {
      for (const tab of ribbonCensus[application]) {
        for (const group of tab.groups) {
          const survivors = essentialCommands(group);
          expect(
            survivors.length,
            `${application}/${tab.id}/${group.label} declares ${String(survivors.length)} ` +
              'essential commands. A survivor row longer than the ceiling is a ribbon again.',
          ).toBeLessThanOrEqual(essentialCommandLimit);
        }
      }
    }
  });

  it('gives every survivor an icon, because a collapsed group has no room for a label', () => {
    for (const application of ribbonApplicationNames) {
      for (const tab of ribbonCensus[application]) {
        for (const group of tab.groups) {
          for (const command of essentialCommands(group)) {
            expect(
              command.icon,
              `${command.id} is essential and has no icon`,
            ).toBeDefined();
          }
        }
      }
    }
  });
});
