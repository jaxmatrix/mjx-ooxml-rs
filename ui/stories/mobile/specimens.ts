/**
 * MJXOFF-194's specimens, and the fixture the whole demotion argument rests on.
 *
 * Every figure a caption prints here is computed from the same functions the gates read, so a
 * caption in the catalogue can never say a different number from the one a test asserts.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { typeRoleClass } from '../../src/foundations/typography.ts';
import {
  commandBarOrder,
  commandBarPartition,
  demotionCost,
  mobileFormFactors,
  naiveCommandBarPartition,
  thumbReachBlockFraction,
  type MobileCommand,
  type MobileFormFactor,
} from '../../src/mobile/mobile-model.ts';
import { sheetDetentNames, sheetDetents } from '../../src/mobile/sheet-detents.ts';
import type { StoryState } from '../../src/story/conventions.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';

/** A note above a specimen, so an auditor knows what they are looking at. */
export function note(text: string): TemplateResult {
  return html`
    <p
      class=${typeRoleClass('body')}
      style="margin:0;padding:calc(var(--mjx-density-gutter) * 2);color:var(--theme-text-primary);max-inline-size:64ch"
    >
      ${text}
    </p>
  `;
}

/** A caption under a specimen, in the measured-figure style the catalogue uses. */
export function caption(lines: readonly string[]): TemplateResult {
  return html`
    <ul
      class=${typeRoleClass('dense')}
      style="margin:0;padding:var(--mjx-density-gutter) calc(var(--mjx-density-gutter) * 3);
             color:var(--theme-text-secondary)"
    >
      ${lines.map((line) => html`<li>${line}</li>`)}
    </ul>
  `;
}

/**
 * A phone-shaped stage with the document above and the bar pinned to its block end.
 *
 * The bar is at the bottom because that is where reach is, and the stage is tall because a bar
 * shown floating in the middle of a page tells an auditor nothing about whether they could hit it.
 */
export function phoneStage(...content: TemplateResult[]): TemplateResult {
  return html`
    <div
      style="min-block-size:30rem;display:flex;flex-direction:column;justify-content:flex-end;
             gap:0;background:var(--theme-background)"
    >
      ${content}
    </div>
  `;
}

/** Something for a bar to sit under. */
export function documentArea(text: string): TemplateResult {
  return html`
    <div
      class=${typeRoleClass('body')}
      style="flex:1 1 auto;padding:calc(var(--mjx-density-gutter) * 2);
             color:var(--theme-text-secondary);overflow:auto"
    >
      ${text}
    </div>
  `;
}

/**
 * **The fixture — and it is chosen to be discriminating, not to be convenient.**
 *
 * It is Word's Home tab reduced to what a phone bar would carry, declared in the order the *ribbon*
 * lays it out: Clipboard first, because that is where Office puts it, and Clipboard is `secondary`
 * — a group whose verbs are on the keyboard anyway. So declaration order and ladder order genuinely
 * disagree, and `tests/mobile.test.ts` asserts that they do before it asserts anything about which
 * of the two is better. A fixture on which the naive answer and the real one coincide would have
 * turned the whole demotion suite into a test of nothing, which is the failure MJXOFF-193's
 * *the tight fixture really is tight* exists to prevent, met a second time.
 *
 * Two commands carry `hasPopup`, so demotion rule 1 has something to refuse, and Font contributes
 * four essential commands so the per-group ceiling of three has something to bite on.
 */
export const wordPhoneCommands: readonly MobileCommand[] = [
  {
    id: 'paste',
    label: 'Paste',
    icon: 'clipboard-paste',
    group: 'Clipboard',
    priority: 'secondary',
    essential: true,
    hasPopup: false,
  },
  {
    id: 'cut',
    label: 'Cut',
    icon: 'cut',
    group: 'Clipboard',
    priority: 'secondary',
    essential: true,
    hasPopup: false,
  },
  {
    id: 'copy',
    label: 'Copy',
    icon: 'copy',
    group: 'Clipboard',
    priority: 'secondary',
    essential: true,
    hasPopup: false,
  },
  {
    id: 'bold',
    label: 'Bold',
    icon: 'text-bold',
    group: 'Font',
    priority: 'primary',
    essential: true,
    hasPopup: false,
  },
  {
    id: 'italic',
    label: 'Italic',
    icon: 'text-italic',
    group: 'Font',
    priority: 'primary',
    essential: true,
    hasPopup: false,
  },
  {
    id: 'underline',
    label: 'Underline',
    icon: 'text-underline',
    group: 'Font',
    priority: 'primary',
    essential: true,
    hasPopup: false,
  },
  {
    id: 'font-colour',
    label: 'Font colour',
    icon: 'radio-button',
    group: 'Font',
    priority: 'primary',
    essential: true,
    hasPopup: true,
  },
  {
    id: 'align-left',
    label: 'Align left',
    icon: 'text-align-left',
    group: 'Paragraph',
    priority: 'standard',
    essential: true,
    hasPopup: false,
  },
  {
    id: 'align-centre',
    label: 'Centre',
    icon: 'text-align-center',
    group: 'Paragraph',
    priority: 'standard',
    essential: true,
    hasPopup: false,
  },
  {
    id: 'bullets',
    label: 'Bullets',
    icon: 'add',
    group: 'Paragraph',
    priority: 'standard',
    essential: false,
    hasPopup: true,
  },
  {
    id: 'styles',
    label: 'Styles',
    icon: 'slide-layout',
    group: 'Styles',
    priority: 'ancillary',
    essential: false,
    hasPopup: true,
  },
  {
    id: 'find',
    label: 'Find',
    icon: 'search',
    group: 'Editing',
    priority: 'ancillary',
    essential: true,
    hasPopup: false,
  },
];

/** The demotion readout a story prints, computed rather than typed. */
export function demotionCaption(factor: MobileFormFactor): readonly string[] {
  const slots = mobileFormFactors[factor].visibleSlots;
  const real = commandBarPartition(wordPhoneCommands, slots);
  const naive = naiveCommandBarPartition(commandBarOrder(wordPhoneCommands), slots);
  return [
    `${factor}: ${String(slots)} visible slots.`,
    `Visible, in ladder order: ${real.visible.map((command) => command.label).join(', ')}.`,
    `Behind them, in the same rail: ${real.overflow.map((command) => command.label).join(', ')}.`,
    `Demotion cost — the ladder's answer ${String(demotionCost(real))}, ` +
      `the answer that just takes the first ${String(slots)} ${String(demotionCost(naive))}.`,
    `Nothing is removed: ${String(real.visible.length + real.overflow.length)} commands in, ` +
      `${String(wordPhoneCommands.length)} out.`,
  ];
}

/** The detent readout, likewise. */
export function detentCaption(): readonly string[] {
  return sheetDetentNames.map(
    (name) =>
      `${name}: ${(sheetDetents[name].fraction * 100).toFixed(0)} % of the boundary — ${sheetDetents[name].use}`,
  );
}

/** A long list, so a sheet has something inside it that scrolls. */
export function longList(rows: number): TemplateResult {
  return html`
    <ol
      class=${typeRoleClass('body')}
      style="margin:0;padding-inline-start:3ch;color:var(--theme-text-primary)"
    >
      ${Array.from(
        { length: rows },
        (_unused, index) => html`<li style="padding-block:var(--mjx-density-step)">
          Paragraph style ${String(index + 1)}
        </li>`,
      )}
    </ol>
  `;
}

/** Token dependencies shared by every mobile story. */
export const mobileTokenDependencies: readonly TokenPath[] = [
  'theme.light.surface',
  'theme.light.surfaceRaised',
  'theme.light.border',
  'theme.light.borderSubtle',
  'theme.light.textPrimary',
  'theme.light.textSecondary',
  'radius.control',
  'radius.panel',
  'duration.transition',
];

/** The states matrix both bars publish. */
export function barStates(): readonly StoryState[] {
  return [
    { name: 'phone portrait', description: 'Four visible slots, spanning the container.' },
    {
      name: 'phone landscape',
      description: 'Six visible slots, one spacing unit of block padding, still spanning.',
    },
    { name: 'small tablet', description: 'Eight visible slots, floating clear of the edges.' },
    { name: 'overflow open', description: 'The same rail, as a wrapped grid above the bar.' },
    { name: 'command held', description: 'A command under a finger or a pointer.' },
    { name: 'roving focus', description: 'One tab stop; the arrow keys move it along the rail.' },
  ];
}

/** The reachability readout. */
export function reachCaption(viewportBlock: number): readonly string[] {
  const threshold = viewportBlock * (1 - thumbReachBlockFraction);
  return [
    `Thumb reach is the bottom ${(thumbReachBlockFraction * 100).toFixed(0)} % of the viewport's ` +
      'block axis.',
    `On a ${String(viewportBlock)} px-tall screen that is everything at or below ` +
      `${threshold.toFixed(0)} px.`,
    'Every primary action must lie WHOLLY inside that band — half a button inside it is a button ' +
      'a person aims at and misses.',
  ];
}
