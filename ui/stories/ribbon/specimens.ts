/**
 * The pieces both ribbon story files build from.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed — the same
 * split `stories/controls/matrix.ts` makes, and for the same reason. A specimen written out twice
 * is two specimens that drift.
 */

import { html, type TemplateResult } from 'lit';

import { typeRoleClass } from '../../src/foundations/typography.ts';
import {
  demotionRules,
  groupPriorities,
  groupPriorityNames,
  type GroupPriority,
} from '../../src/ribbon/ribbon-model.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';
import {
  specimenCommands,
  wordTabHomeGroups,
  type SpecimenCommand,
  type SpecimenGroup,
} from '../../dev/word-tab-home.ts';

/**
 * Everything the ribbon reads, as one list.
 *
 * Hand-written rather than derived, unlike `stories/controls/matrix.ts`'s, and the difference is
 * worth stating: a control's blast radius *is* its state table, which is a machine-readable thing.
 * A ribbon's is the state table **plus** the surfaces its two popups sit on, the radii of its
 * chips, and the honey pair a contextual tab is told apart by. `storyConventions` still checks
 * every name against the generated table, so a stale one throws with the name in the message.
 */
export const ribbonTokenDependencies: readonly TokenPath[] = [
  'duration.transition',
  'ease.outSoft',
  'fontWeight.bold',
  'fontWeight.medium',
  'leading.snug',
  'leading.tight',
  'radius.chip',
  'radius.control',
  'radius.panel',
  'shadow.lift',
  'spacing',
  'text.sm',
  'text.xs',
  'theme.light.accentBorder',
  'theme.light.accentPressed',
  'theme.light.accentSurface',
  'theme.light.border',
  'theme.light.borderSubtle',
  'theme.light.secondaryAccent',
  'theme.light.secondarySurface',
  'theme.light.surface',
  'theme.light.surfaceRaised',
  'theme.light.textPrimary',
  'theme.light.textSecondary',
];

/** The states an auditor must be able to see on a ribbon. */
export const ribbonStatesMatrix: readonly { name: string; description: string }[] = [
  { name: 'expanded', description: 'Tabs and the selected tab’s groups. The ordinary state.' },
  { name: 'collapsed to tabs', description: 'Tabs alone; the commands are one press away.' },
  { name: 'hidden', description: 'Only the button that brings the ribbon back.' },
  { name: 'simplified', description: 'The single-row form: every group at most reduced.' },
  {
    name: 'group · full',
    description: 'A group with room: large commands, the name under them, the launcher beside it.',
  },
  {
    name: 'group · reduced',
    description: 'Compact density, a large command laid out sideways, two rows of commands.',
  },
  {
    name: 'group · collapsed',
    description:
      'One button carrying the name, the essential commands beside it, and everything else in a popup.',
  },
  { name: 'group · collapsed and open', description: 'The popup showing, with focus inside it.' },
  { name: 'tab strip · strip', description: 'Every tab side by side.' },
  { name: 'tab strip · picker', description: 'One button naming the selected tab; the list is a popup.' },
  { name: 'tab · selected', description: 'The accent tint and a bold label — the pressed paint.' },
  {
    name: 'tab · contextual',
    description: 'Under a titled honey band, and announced with its set’s name in its own.',
  },
];

/** The keyboard contract, written once because both story files promise the same one. */
export const ribbonKeyboard: readonly { keys: string; does: string }[] = [
  { keys: 'Tab', does: 'Enters the tab strip at the selected tab, and leaves it on the next press. Exactly one tab is a tab stop.' },
  { keys: 'Arrow Right / Arrow Left', does: 'Moves along the tabs and selects as it goes — automatic activation, which is what a ribbon does.' },
  { keys: 'Home / End', does: 'The first and last tab.' },
  { keys: 'Arrow Down / Arrow Up', does: 'The same, in the narrow-width picker, where the list is vertical.' },
  { keys: 'Enter or Space (a collapsed group)', does: 'Opens the group’s popup. Focus does not move.' },
  { keys: 'Arrow Down (a collapsed group)', does: 'Opens it and puts focus on the first command.' },
  { keys: 'Tab (inside an open group)', does: 'Cycles within the group. Focus cannot leave an open popup.' },
  { keys: 'Escape', does: 'Closes the popup and returns focus to the button that opened it — the group’s trigger, or the tab picker.' },
];

export const ribbonScreenReader =
  'The tab strip announces “Ribbon, tab list”, then each tab as “Home, tab, selected, 1 of 10”. ' +
  'A contextual tab announces its set in its own name — “Design, Table Tools, tab” — because the ' +
  'coloured band that says so visually is a picture. A set appearing announces “Table Tools tab ' +
  'set available” politely, and closing announces “Table Tools tab set closed”. A collapsed group ' +
  'announces “Font, collapsed, button” and its popup announces “Font, group”.';

/** One command, as whichever archetype it is. */
export function commandSpecimen(command: SpecimenCommand): TemplateResult {
  const slot = command.essential === true ? 'essential' : undefined;
  if (command.toggle === true) {
    return html`<mjx-toggle-button
      slot=${slot ?? ''}
      label=${command.label}
      icon=${command.icon ?? ''}
      size=${command.size ?? 'small'}
    ></mjx-toggle-button>`;
  }
  return html`<mjx-button
    slot=${slot ?? ''}
    label=${command.label}
    icon=${command.icon ?? ''}
    size=${command.size ?? 'small'}
  ></mjx-button>`;
}

/** One group of the Word specimen, with all of its commands. */
export function groupSpecimen(group: SpecimenGroup): TemplateResult {
  return html`<mjx-ribbon-group label=${group.label} priority=${group.priority}>
    ${specimenCommands(group).map((command) => commandSpecimen(command))}
    ${group.dialogLauncher
      ? html`<mjx-dialog-launcher
          slot="dialog-launcher"
          label=${`${group.label} settings`}
        ></mjx-dialog-launcher>`
      : ''}
  </mjx-ribbon-group>`;
}

/** Word's `TabHome`, all thirteen groups of it. */
export function wordTabHomeGroupsTemplate(): TemplateResult[] {
  return wordTabHomeGroups.map((group) => groupSpecimen(group));
}

/**
 * A small group for the ladder story: four commands, one of them essential.
 *
 * Deliberately identical apart from the priority, so the story shows the ladder and nothing else.
 */
export function ladderGroup(priority: GroupPriority): TemplateResult {
  const name = priority.charAt(0).toUpperCase() + priority.slice(1);
  return html`<mjx-ribbon-group label=${name} priority=${priority}>
    <mjx-toggle-button
      slot="essential"
      label=${`${name} bold`}
      icon="text-bold"
      size="icon"
    ></mjx-toggle-button>
    <mjx-button label=${`${name} paste`} icon="folder-open" size="large"></mjx-button>
    <mjx-button label=${`${name} find`} icon="search"></mjx-button>
    <mjx-button label=${`${name} table`} icon="table"></mjx-button>
    <mjx-dialog-launcher
      slot="dialog-launcher"
      label=${`${name} settings`}
    ></mjx-dialog-launcher>
  </mjx-ribbon-group>`;
}

/** The four ladder groups, one per priority, in the order they give way. */
export function ladderGroups(): TemplateResult[] {
  return groupPriorityNames.map((priority) => ladderGroup(priority));
}

const noteStyle =
  'margin:0 0 var(--mjx-density-gutter);color:var(--theme-text-secondary);max-inline-size:60ch';

/** A short explanation above a specimen, so an auditor knows what they are looking at. */
export function note(text: string): TemplateResult {
  return html`<p class=${typeRoleClass('body')} style=${noteStyle}>${text}</p>`;
}

const tableStyle =
  'border-collapse:collapse;margin:var(--mjx-density-gutter) 0;max-inline-size:80ch';
const cellStyle =
  'border:1px solid var(--theme-border);padding:var(--mjx-density-step) var(--mjx-density-gutter);text-align:start;vertical-align:top';

/** The priority ladder, drawn as the table it is. */
export function ladderTable(): TemplateResult {
  return html`
    <table class=${typeRoleClass('body')} style=${tableStyle}>
      <caption class=${typeRoleClass('dense')} style="text-align:start;padding-block-end:var(--mjx-density-step)">
        The priority ladder, in container pixels. A group declares one of these; the widths are the
        ladder's, not the group's.
      </caption>
      <thead>
        <tr>
          <th scope="col" style=${cellStyle}>Priority</th>
          <th scope="col" style=${cellStyle}>Reduced at or below</th>
          <th scope="col" style=${cellStyle}>Collapsed at or below</th>
          <th scope="col" style=${cellStyle}>What it means</th>
        </tr>
      </thead>
      <tbody>
        ${groupPriorityNames.map(
          (priority) => html`
            <tr>
              <th scope="row" style=${cellStyle}>${priority}</th>
              <td style=${cellStyle}>${groupPriorities[priority].reduceAtOrBelow}</td>
              <td style=${cellStyle}>${groupPriorities[priority].collapseAtOrBelow}</td>
              <td style=${cellStyle}>${groupPriorities[priority].use}</td>
            </tr>
          `,
        )}
      </tbody>
    </table>
  `;
}

/** The demotion rules, drawn from `demotionRules` so the story cannot disagree with the model. */
export function demotionTable(): TemplateResult {
  return html`
    <table class=${typeRoleClass('body')} style=${tableStyle}>
      <caption class=${typeRoleClass('dense')} style="text-align:start;padding-block-end:var(--mjx-density-step)">
        Which commands survive a collapse, and by what criterion. All four must hold.
      </caption>
      <thead>
        <tr>
          <th scope="col" style=${cellStyle}>Rule</th>
          <th scope="col" style=${cellStyle}>Because</th>
          <th scope="col" style=${cellStyle}>Checked by</th>
        </tr>
      </thead>
      <tbody>
        ${demotionRules.map(
          (entry) => html`
            <tr>
              <th scope="row" style=${cellStyle}>${entry.rule}</th>
              <td style=${cellStyle}>${entry.because}</td>
              <td style=${cellStyle}>${entry.checkedBy}</td>
            </tr>
          `,
        )}
      </tbody>
    </table>
  `;
}
