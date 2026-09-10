/**
 * The feedback specimens, and the captions that are **measured rather than typed**.
 *
 * Every number below is computed by the same functions `tests/feedback.test.ts` sweeps — the delays
 * out of the timing table, the tone contrasts and the colour separation out of the generated
 * palette. MJXOFF-269 is the standing record of what a caption that quoted a figure is worth once
 * the palette moves: the figure stays and the screen changes. Reading them here means a re-seed
 * changes the caption and the gate in the same commit.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import {
  feedbackTimingMilliseconds,
  feedbackTimingMultiples,
  feedbackTimingNames,
  formatRatio,
  toastEdgeContrast,
  toastSignals,
  toastToneNames,
  toastTones,
  toneColourSeparation,
  type FeedbackTiming,
} from '../../src/feedback/feedback-model.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';
import type { StoryState } from '../../src/story/conventions.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';
import type { ColorScheme } from '../../tokens/tokens.ts';

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

/** A page-sized stage, so a pinned toast region has a boundary to be pinned inside. */
export function stage(...content: TemplateResult[]): TemplateResult {
  return html`
    <div
      style="min-block-size:30rem;display:flex;flex-direction:column;gap:var(--mjx-density-gutter);
             padding:calc(var(--mjx-density-gutter) * 2);background:var(--theme-background)"
    >
      ${content}
    </div>
  `;
}

/** A run of document text a selection can be made inside. */
export function documentBody(paragraphs = 4): TemplateResult {
  const lines = Array.from({ length: paragraphs }, (_unused, index) => index);
  return html`
    <div
      class=${typeRoleClass('body')}
      style="display:grid;gap:var(--mjx-density-gutter);color:var(--theme-text-primary)"
    >
      ${lines.map(
        (index) => html`
          <p style="margin:0">
            Paragraph ${String(index + 1)} — a run of the document the feedback sits over, so that
            “it never covers the selection” is a claim with something to be false about.
          </p>
        `,
      )}
    </div>
  `;
}

/** A painted selection rectangle, drawn in the document's own selection token. */
export function selectionRun(id: string, text: string): TemplateResult {
  return html`
    <mark
      id=${id}
      class=${typeRoleClass('body')}
      style="background:var(--document-light-selection-fill);color:var(--theme-text-primary);
             padding-inline:var(--spacing);border-radius:var(--radius-chip)"
      >${text}</mark
    >
  `;
}

/** The six spans, with the ratio each is and the milliseconds it currently resolves to. */
export function timingReport(): TemplateResult {
  return html`
    <div
      class=${typeRoleClass('dense')}
      style="display:grid;gap:var(--mjx-density-step);padding:calc(var(--mjx-density-gutter) * 2);
             color:var(--theme-text-primary)"
    >
      <p style="margin:0">
        Not one of these is a number in a component. Each is a multiple of the platform’s single
        duration token, so a host that slows the interface down for someone who needs longer to read
        slows every one of them with it.
      </p>
      ${feedbackTimingNames.map(
        (span: FeedbackTiming) => html`
          <p style="margin:0">
            <strong>${span}</strong> — ${String(feedbackTimingMultiples[span])} ×
            <code>--duration-transition</code>, which resolves today to
            ${String(feedbackTimingMilliseconds(span))} ms.
          </p>
        `,
      )}
    </div>
  `;
}

const schemes: readonly ColorScheme[] = ['light', 'dark'];

/**
 * The tone report — and the measurement that is the whole argument for it.
 *
 * The separation figure is the one worth reading twice: it is how far apart two tones' edges are in
 * *luminance*, which is the only dimension a contrast ratio can see. A number near 1 means a
 * contrast gate would call those two colours the same.
 */
export function toneReport(): TemplateResult {
  return html`
    <div
      class=${typeRoleClass('dense')}
      style="display:grid;gap:var(--mjx-density-step);padding:calc(var(--mjx-density-gutter) * 2);
             color:var(--theme-text-primary)"
    >
      <p style="margin:0">
        This brand has one alarm colour and no red, and inventing one would override the palette of
        whoever opens the editor. So a tone is carried by four signals and the colour is the
        weakest of them.
      </p>
      ${schemes.map(
        (scheme) => html`
          <p style="margin:0">
            <strong>${scheme}</strong> —
            ${toastToneNames
              .map(
                (tone) =>
                  `${tone} ${toastTones[tone].edge} ${formatRatio(toastEdgeContrast(tone, scheme))} against the card`,
              )
              .join('; ')}.
          </p>
        `,
      )}
      <p style="margin:0">
        And why the colour cannot be the signal: warning against success separates by only
        ${formatRatio(toneColourSeparation('warning', 'success', 'light'))} in the light scheme and
        ${formatRatio(toneColourSeparation('warning', 'success', 'dark'))} in the dark one. What
        actually tells them apart is
        ${toastToneNames.map((tone) => `${tone} → ${toastSignals(tone).join(' · ')}`).join('; ')}.
      </p>
    </div>
  `;
}

/** The states matrix, drawn from the tone table so a new tone appears without anybody remembering. */
export function toneStates(): StoryState[] {
  return toastToneNames.map((tone) => ({
    name: tone,
    description: `${toastTones[tone].use} Announced ${toastTones[tone].politeness}; ${
      toastTones[tone].dwell === 'persistent' ? 'never leaves on its own' : 'leaves on its own'
    }.`,
  }));
}

/** Every token this child's components read. */
export const feedbackTokenDependencies: readonly TokenPath[] = [
  'theme.light.surface',
  'theme.light.surfaceRaised',
  'theme.light.background',
  'theme.light.border',
  'theme.light.borderSubtle',
  'theme.light.textPrimary',
  'theme.light.textSecondary',
  'theme.light.accent',
  'theme.light.accentPressed',
  'theme.light.secondaryAccent',
  'theme.dark.surfaceRaised',
  'theme.dark.background',
  'theme.dark.border',
  'theme.dark.borderSubtle',
  'theme.dark.textPrimary',
  'theme.dark.textSecondary',
  'theme.dark.accent',
  'theme.dark.secondaryAccent',
  'document.light.selectionFill',
  'radius.chip',
  'radius.control',
  'radius.card',
  'radius.panel',
  'shadow.lift',
  'spacing',
  'duration.transition',
  'ease.outSoft',
  'text.xs',
  'text.sm',
  'font.sans',
  'font.serif',
];

/** The keyboard rows, shared by the five story files. */
export const feedbackKeyboard = [
  {
    keys: 'Tab',
    does: 'Enters a mini toolbar once and leaves it once — it holds a single roving stop. Reaches a toast’s action and its dismissal, and an empty state’s action.',
  },
  {
    keys: 'Arrow Left / Arrow Right',
    does: 'Moves between a mini toolbar’s commands, mirrored under right-to-left. Home and End take the first and the last.',
  },
  {
    keys: 'Escape',
    does: 'Dismisses a mini toolbar, returning focus only if the keyboard was inside it; dismisses a screentip and moves nothing at all.',
  },
  {
    keys: 'Enter / Space',
    does: 'Activates whatever the keyboard is on: a toolbar command, a toast’s action or dismissal, an empty state’s action.',
  },
  {
    keys: 'Focus (any means)',
    does: 'Shows a screentip immediately. A pointer waits; a keyboard arrived on purpose and does not.',
  },
];

/** What a screen reader announces. */
export const feedbackScreenReader =
  'A mini toolbar announces “toolbar” with its name, and each command announces its own name and, ' +
  'for a toggle, whether it is pressed. A screentip is never announced as a tip: its text is the ' +
  'trigger’s accessible description, present whether or not the tip is drawn, which is why a ' +
  'delay that exists for eyes costs a screen-reader user nothing. A toast is announced once, into ' +
  'the polite region or the assertive one according to its tone and never both — an error ' +
  'interrupts and a confirmation waits. A determinate progress bar announces its name and its ' +
  'percentage; an indeterminate one announces its name and “working”, and carries no value at all, ' +
  'because a number nobody knows is a number nobody should be told. An empty state announces ' +
  'itself politely when it appears, named by its heading, and its illustration is silent.';
