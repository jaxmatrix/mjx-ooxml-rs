/**
 * The furniture specimens, and the captions that are **measured rather than typed**.
 *
 * Every figure a caption prints here is computed from the same functions the gates sweep, so a
 * caption in the catalogue can never say a different number from the one a test asserts, and a
 * re-seed of the palette moves both together. `<mjx-color-picker>` established that shape and every
 * child since has followed it.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { typeRoleClass } from '../../src/foundations/typography.ts';
import {
  fitToPagePercent,
  fitToWidthPercent,
  letterPagePixels,
  scrollbarContrastCaption,
  statusPriorities,
  statusPriorityNames,
  zoomSteps,
} from '../../src/furniture/furniture-model.ts';
import {
  ScrollbarModel,
  minimumThumbFraction,
  thumbFraction,
} from '../../src/furniture/scroll-model.ts';
import { containerPresets } from '../../src/harness/presets.ts';
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

/** A page-sized stage, so a scrollbar and a splitter have something to be beside. */
export function stage(...content: TemplateResult[]): TemplateResult {
  return html`
    <div
      style="min-block-size:28rem;display:flex;flex-direction:column;
             gap:var(--mjx-density-gutter);padding:calc(var(--mjx-density-gutter) * 2);
             background:var(--theme-background)"
    >
      ${content}
    </div>
  `;
}

/** The token dependencies every furniture story declares. One list, four components. */
export const furnitureTokenDependencies: readonly TokenPath[] = [
  'theme.light.background',
  'theme.light.surface',
  'theme.light.surfaceRaised',
  'theme.light.border',
  'theme.light.borderSubtle',
  'theme.light.textPrimary',
  'theme.light.textSecondary',
  'theme.light.accent',
  'theme.light.accentPressed',
  'theme.light.secondaryAccent',
  'theme.dark.background',
  'theme.dark.textPrimary',
  'theme.dark.textSecondary',
  'theme.dark.accentPressed',
  'theme.dark.secondaryAccent',
  'radius.chip',
  'radius.control',
  'spacing',
  'duration.transition',
  'ease.outSoft',
  'text.xs',
  'text.sm',
  'font.sans',
];

// ── the status bar ───────────────────────────────────────────────────────────

/** The states an auditor must be able to see on a status bar. Drawn from the ladder itself. */
export function statusStates(): StoryState[] {
  return [
    {
      name: 'everything on the bar',
      description: 'At a desktop width all four priorities are shown and the overflow is absent.',
    },
    ...statusPriorityNames.map((priority) => ({
      name: `${priority} dropped`,
      description:
        statusPriorities[priority].dropAtOrBelow === undefined
          ? `Never dropped. ${statusPriorities[priority].use}`
          : `At or below ${String(statusPriorities[priority].dropAtOrBelow)}px it moves into the overflow. ${statusPriorities[priority].use}`,
    })),
    {
      name: 'overflow open',
      description:
        'The disclosure holds the segments the bar could not fit. They are the same elements — nothing is rendered twice and nothing is lost.',
    },
    {
      name: 'announced',
      description:
        'A segment declaring polite or assertive puts its change into the matching live region. A page number declares neither, on purpose.',
    },
  ];
}

/** What a status bar does with a keyboard. */
export const statusKeyboard = [
  { keys: 'Tab', does: 'Reaches the overflow disclosure when the bar has one. The readings themselves are not focusable — they are text.' },
  { keys: 'Enter / Space', does: 'Opens and closes the overflow panel.' },
];

/** What a status bar announces. */
export const statusScreenReader =
  'The bar is a group named “Document status”. Each reading is announced when a person reaches it — “Page 4 of 20” — and a change to it is announced only if the segment asked to be: politely into the status region, assertively into the alert region, and, by default, not at all.';

/** The caption that says which priorities are on the bar at each preset. Computed, never typed. */
export function ladderCaption(): string[] {
  return (['desktop', 'tablet', 'phone'] as const).map((preset) => {
    const width = containerPresets[preset];
    const shown = statusPriorityNames.filter((priority) => {
      const drop = statusPriorities[priority].dropAtOrBelow;
      return drop === undefined || width > drop;
    });
    return `${preset} (${String(width)}px): ${shown.length === 0 ? 'nothing' : shown.join(', ')}`;
  });
}

// ── the zoom control ─────────────────────────────────────────────────────────

/** The states an auditor must be able to see on a zoom control. */
export const zoomStates: StoryState[] = [
  { name: 'resting', description: 'At 100 %, both stepping commands available.' },
  { name: 'at the minimum', description: 'At 10 % the zoom-out command is disabled — a limit a person can see, rather than a button that stopped responding.' },
  { name: 'at the maximum', description: 'At 500 % the zoom-in command is disabled.' },
  { name: 'fitted', description: 'After Fit width or Fit page, that command is disabled because the document is already at it.' },
  { name: 'invalid', description: 'A typed string the readout cannot read: the text stays, nothing is committed, and the field says so three ways at once.' },
  { name: 'clamped', description: 'A typed 700 becomes 500. Out of range is not a failure — it is a number a person meant.' },
  { name: 'unavailable', description: 'Greyed, still reachable, and carrying the reason.' },
];

/** What a zoom control does with a keyboard. */
export const zoomKeyboard = [
  { keys: 'Tab', does: 'Moves through zoom out, the slider, zoom in, the readout, Fit width and Fit page.' },
  { keys: 'Arrow Left / Arrow Right', does: 'On the slider, moves by one per cent.' },
  { keys: 'Page Up / Page Down', does: 'On the slider, moves by a tenth of the range.' },
  { keys: 'Home / End', does: 'On the slider, 10 % and 500 % exactly.' },
  { keys: 'Enter', does: 'In the readout, commits what was typed — or refuses it and says why.' },
  { keys: 'Escape', does: 'In the readout, abandons what was typed and goes back to the committed zoom. The only way out of an invalid field.' },
];

/** What a zoom control announces. */
export const zoomScreenReader =
  'The slider announces “Zoom, slider, 100 %”, and its value text is the percentage rather than a bare number. The readout announces “Zoom percentage, edit, 100%”, and a string it cannot read makes it invalid and reads the reason out beneath it.';

/** The stops, and the two fits, as a caption. Computed from the model. */
export function zoomCaption(viewport: { width: number; height: number }): string[] {
  return [
    `Stops: ${zoomSteps.join(', ')} per cent.`,
    `A page is ${String(letterPagePixels.width)} × ${String(letterPagePixels.height)} CSS pixels — US Letter at 96 dpi.`,
    `In a ${String(viewport.width)} × ${String(viewport.height)} viewport: fit width is ${String(fitToWidthPercent(viewport))} %, fit page is ${String(fitToPagePercent(viewport))} %.`,
    'Both are floored, never rounded: a fit rounded up overflows by a fraction of a pixel and produces the scrollbar the command exists to avoid.',
  ];
}

// ── the scrollbar ────────────────────────────────────────────────────────────

/** The states an auditor must be able to see on a scrollbar. */
export const scrollbarStates: StoryState[] = [
  { name: 'estimated', description: 'A document just opened. Every page height is a guess, and the announcement says so.' },
  { name: 'corrected', description: 'Real page heights have arrived. The thumb is a different size and the reader has not moved.' },
  { name: 'barely overflowing', description: 'Content only slightly taller than the viewport: an almost-full thumb with a little travel.' },
  { name: 'massively overflowing', description: 'A five-hundred-page document, where the thumb is on its minimum size rather than sub-pixel.' },
  { name: 'not scrollable', description: 'Content that fits. The thumb is hidden and the track is out of the tab order — a full-length thumb that cannot move looks exactly like a working scrollbar.' },
  { name: 'marked', description: 'Search hits, comments and tracked changes in three lanes. Never told apart by colour alone.' },
];

/** What a scrollbar does with a keyboard. */
export const scrollbarKeyboard = [
  { keys: 'Tab', does: 'Focuses the track, which is the scrollbar.' },
  { keys: 'Arrow Up / Arrow Down', does: 'Moves by a tenth of the viewport.' },
  { keys: 'Page Up / Page Down', does: 'Moves by a whole viewport.' },
  { keys: 'Home / End', does: 'The top of the document and the end of its scrollable extent.' },
];

/** What a scrollbar announces. */
export const scrollbarScreenReader =
  'A scrollbar named for the document, whose value text is the page a reader is on rather than a percentage — “Page 4 of 20”, with “(estimated)” while the page count is still a guess. The channel’s contents are summarised into a polite region: “3 search results, 1 comment”.';

/** The thumb sizes a set of ratios produces. Computed, so the caption cannot drift. */
export function thumbCaption(): string[] {
  const rows: { viewport: number; total: number; what: string }[] = [
    { viewport: 800, total: 800, what: 'content that exactly fits' },
    { viewport: 800, total: 900, what: 'content that barely overflows' },
    { viewport: 800, total: 4000, what: 'a five-viewport document' },
    { viewport: 800, total: 400_000, what: 'a five-hundred-page document' },
  ];
  return [
    ...rows.map(
      (row) =>
        `${row.what} (${String(row.viewport)} of ${String(row.total)}): thumb is ${(thumbFraction(row.viewport, row.total) * 100).toFixed(1)} % of the track.`,
    ),
    `The floor is ${(minimumThumbFraction * 100).toFixed(0)} % — a thumb sized honestly over a very long document is under a pixel tall, which is a control nobody can grab.`,
  ];
}

/** The measured contrast of every scrollbar pair, in both schemes. */
export function scrollbarContrast(): string[] {
  return [scrollbarContrastCaption('light'), scrollbarContrastCaption('dark')];
}

/** A model with the shape the specimens use, so a story and a gate build the same document. */
export function specimenModel(pages: number, pageHeight: number): ScrollbarModel {
  return ScrollbarModel.fromExtent({ pages, pageHeight, precision: 'estimated' });
}

// ── the splitter ─────────────────────────────────────────────────────────────

/** The states an auditor must be able to see on a splitter. */
export const splitterStates: StoryState[] = [
  { name: 'resting', description: 'A divider between two regions, at the fraction the author declared.' },
  { name: 'focused', description: 'The platform focus ring, on a hit target that clears 24 CSS pixels even in compact density.' },
  { name: 'at a minimum', description: 'Dragged or keyed to the narrowest either side may be. Each side declares its own minimum.' },
  { name: 'collapsed', description: 'Double-clicked, or Enter or Space. The leading region is gone and the line says so.' },
  { name: 'restored', description: 'Collapsed a second time. It goes back to where the person had it, not to a default nobody chose.' },
];

/** What a splitter does with a keyboard. */
export const splitterKeyboard = [
  { keys: 'Tab', does: 'Focuses the separator.' },
  { keys: 'Arrow Left / Arrow Right', does: 'Moves it by two per cent of the boundary. The arrow that grows the leading region mirrors under right-to-left.' },
  { keys: 'Home / End', does: 'The narrowest and the widest either side allows.' },
  { keys: 'Enter / Space', does: 'Collapses the leading region, and restores it on a second press.' },
];

/** What a splitter announces. */
export const splitterScreenReader =
  'A separator named for what it resizes — “Resize the navigator” — carrying its position as a percentage in aria-valuenow, and “Collapsed” as its value text when it is.';
