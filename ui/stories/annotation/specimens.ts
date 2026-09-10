/**
 * MJXOFF-193's specimens — and the **packing readout**, which is the point of them.
 *
 * The ticket's trap says a hand-made fixture hides the problem, so every review-pane story here
 * carries a live readout of what the packer decided: each card's anchor, where it ended up, and how
 * far it had to move. An auditor selecting a different card watches the whole column re-settle
 * around it, and a story that had stopped packing would be visibly a stack rather than plausibly
 * correct.
 *
 * The second readout is the **connector contract** — the same list, in the shape R11 receives it —
 * because a contract nobody can see is a contract nobody checks.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { typeRoleClass } from '../../src/foundations/typography.ts';
import type { AnnotationAnchorReport } from '../../src/annotation/annotation-model.ts';
import {
  annotationEvents,
  packMarginCards,
  packingCost,
  stackMarginCards,
} from '../../src/annotation/annotation-model.ts';
import { authorColourSlots } from '../../src/annotation/author-colour.ts';
import type { ReviewAnnotation } from '../../src/annotation/review-pane.ts';
import type { StoryState } from '../../src/story/conventions.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';

/** A note above a specimen, so an auditor knows what they are looking at. */
export function note(text: string): TemplateResult {
  return html`
    <p
      class=${typeRoleClass('body')}
      style="margin:0;padding:calc(var(--mjx-density-gutter) * 2);color:var(--theme-text-primary);max-inline-size:72ch"
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

/** A stage with room for a margin column and a readout beside it. */
export function stage(...content: TemplateResult[]): TemplateResult {
  return html`
    <div
      style="min-block-size:36rem;display:flex;flex-direction:column;
             gap:var(--mjx-density-gutter);padding:calc(var(--mjx-density-gutter) * 2);
             background:var(--theme-background)"
    >
      ${content}
    </div>
  `;
}

/**
 * The live readout every review story carries.
 *
 * ⚠ `tabindex="0"`, and it is required rather than decorative. The readout has a maximum height and
 * scrolls, and a scroll container a keyboard cannot reach fails axe's `scrollable-region-focusable`
 * — which a three-hundred-annotation story is the first one here long enough to trip. The harness
 * stage carries the same attribute for the same reason.
 */
export function readout(id: string): TemplateResult {
  return html`
    <output
      id=${id}
      tabindex="0"
      class=${typeRoleClass('dense')}
      style="display:block;white-space:pre-wrap;font-family:var(--font-mono);
             color:var(--theme-text-primary);background:var(--theme-surface-raised);
             border:1px solid var(--theme-border);border-radius:var(--radius-chip);
             padding:var(--mjx-density-gutter);max-block-size:16rem;overflow:auto"
      >select a card to see the column re-settle</output
    >
  `;
}

/** A column of a fixed width, the way a margin actually is. */
export function marginColumn(inner: TemplateResult): TemplateResult {
  return html`
    <div
      style="inline-size:100%;max-inline-size:22rem;block-size:28rem;
             border:1px solid var(--theme-border);border-radius:var(--radius-panel);overflow:hidden"
    >
      ${inner}
    </div>
  `;
}

/** One line per card: where the anchor is, where the card went, and what it cost. */
function describePacking(anchors: readonly AnnotationAnchorReport[]): string {
  if (anchors.length === 0) return 'nothing to place';
  const cards = anchors.map((anchor) => ({
    id: anchor.id,
    anchorTop: anchor.anchorTop,
    extent: anchor.cardExtent,
  }));
  const gap = 12;
  const packed = packMarginCards(cards, { gap, columnTop: 0 });
  const stacked = stackMarginCards(cards, { gap, columnTop: 0 });
  const lines = anchors.map((anchor) => {
    const moved = anchor.cardTop - anchor.anchorTop;
    const direction = moved < 0 ? 'up' : moved > 0 ? 'down' : '--';
    return (
      `${anchor.id.padEnd(5)} anchor ${String(Math.round(anchor.anchorTop)).padStart(5)} ` +
      `card ${String(Math.round(anchor.cardTop)).padStart(5)} ` +
      `${direction} ${String(Math.abs(Math.round(moved))).padStart(4)} ` +
      `slot ${String(anchor.authorSlot)}${anchor.selected ? '  <- selected' : ''}`
    );
  });
  return [
    `packed cost ${String(Math.round(packingCost(packed)))}   ` +
      `a simple stack would cost ${String(Math.round(packingCost(stacked)))}`,
    ...lines,
  ].join('\n');
}

/** The connector contract, in the shape the canvas receives it. */
function describeConnectors(anchors: readonly AnnotationAnchorReport[]): string {
  return anchors
    .slice(0, 12)
    .map(
      (anchor) =>
        `${anchor.id.padEnd(5)} ${anchor.kind.padEnd(13)} ` +
        `anchor ${String(Math.round(anchor.anchorTop)).padStart(5)} -> ` +
        `connector ${String(Math.round(anchor.connectorTop)).padStart(5)} ` +
        `${anchor.side}${anchor.model === undefined ? '' : `  ${anchor.model}`}`,
    )
    .join('\n');
}

/** Wire a pane's two contracts to a readout. */
export function report(paneId: string, readoutId: string, kind: 'packing' | 'connector'): void {
  queueMicrotask(() => {
    const pane = document.getElementById(paneId);
    const output = document.getElementById(readoutId);
    if (!(pane instanceof HTMLElement) || !(output instanceof HTMLElement)) return;
    const write = (anchors: readonly AnnotationAnchorReport[]): void => {
      output.textContent = kind === 'packing' ? describePacking(anchors) : describeConnectors(anchors);
    };
    pane.addEventListener(annotationEvents.anchors, (event) => {
      write((event as CustomEvent<{ anchors: readonly AnnotationAnchorReport[] }>).detail.anchors);
    });
    const initial = (pane as HTMLElement & { anchors?: readonly AnnotationAnchorReport[] }).anchors;
    if (initial !== undefined) write(initial);
  });
}

/** Fill a pane's annotations, which are a property and not markup. */
export function fillAnnotations(paneId: string, annotations: readonly ReviewAnnotation[]): void {
  queueMicrotask(() => {
    const pane = document.getElementById(paneId);
    if (!(pane instanceof HTMLElement)) return;
    (pane as HTMLElement & { annotations: readonly ReviewAnnotation[] }).annotations = annotations;
  });
}

/** Fill a thread's replies and its mention roster. */
export function fillThread(
  threadId: string,
  replies: readonly { id: string; author: string; time: string; text: string }[],
  roster: readonly string[],
): void {
  queueMicrotask(() => {
    const thread = document.getElementById(threadId);
    if (!(thread instanceof HTMLElement)) return;
    const typed = thread as HTMLElement & {
      replies: readonly { id: string; author: string; time: string; text: string }[];
      roster: readonly string[];
    };
    typed.replies = replies;
    typed.roster = roster;
  });
}

/** The states an auditor must be able to see on a card. */
export const cardStates: readonly StoryState[] = [
  { name: 'threaded', description: 'The modern model: replies and a resolved flag both exist.' },
  { name: 'legacy', description: 'A w:comment with no commentsExtended entry: no reply, no resolve.' },
  { name: 'resolved', description: 'A closed conversation. Quieter, never dimmed.' },
  { name: 'selected', description: 'The card the reader is on. It holds its anchor exactly.' },
  { name: 'collapsed', description: 'A thread showing its latest reply and a count of the rest.' },
  { name: 'expanded', description: 'Every reply, each one an article of its own.' },
  { name: 'insertion', description: 'A tracked w:ins, underlined excerpt.' },
  { name: 'deletion', description: 'A tracked w:del, struck-through excerpt.' },
  { name: 'formatting', description: 'A tracked w:rPrChange.' },
  { name: 'move', description: 'A tracked w:moveTo.' },
];

/** The states an auditor must be able to see on the pane. */
export const paneStates: readonly StoryState[] = [
  { name: 'margin', description: 'The column beside the document, cards packed to their anchors.' },
  { name: 'clustered', description: 'Anchors a few lines apart: the packing problem, visible.' },
  { name: 'selected', description: 'A card holding its anchor while the others yield around it.' },
  { name: 'sheet', description: 'The phone presentation: a sheet listing annotations, in flow.' },
  { name: 'empty', description: 'A document with nothing to review.' },
  { name: 'virtualised', description: 'Three hundred annotations, a window of about a dozen.' },
];

/** The tokens a change here would move. Checked against the generated table at runtime. */
export const annotationTokenDependencies: readonly TokenPath[] = [
  ...authorColourSlots.map((slot) => slot.light),
  ...authorColourSlots.map((slot) => slot.dark),
  'theme.light.surface',
  'theme.light.surfaceRaised',
  'theme.light.background',
  'theme.light.border',
  'theme.light.borderSubtle',
  'theme.light.textPrimary',
  'theme.light.textSecondary',
  'theme.light.accentBorder',
  'radius.card',
  'radius.control',
  'radius.chip',
  'radius.phone',
  'spacing',
  'duration.transition',
  'ease.ink',
];

/** The keyboard the feed pattern specifies, in the words a story declares it. */
export const annotationKeyboard = [
  { keys: 'Tab', does: 'Moves into the column, then through the built cards and their controls.' },
  { keys: 'Page Down / Page Up', does: 'Moves a viewport of annotations at a time.' },
  { keys: 'Arrow Down / Arrow Up', does: 'Moves one annotation, for readers arriving from Office.' },
  { keys: 'Control + Home / Control + End', does: 'Moves to the first or last annotation.' },
  { keys: 'Enter / Space', does: "Activates the focused card's button." },
];

/** What a screen reader says, in the words it says them. */
export const annotationScreenReader =
  'The column is announced as a feed named for the pane. Each card is an article named for its ' +
  'model, its author and its time — “conversation started by Ada Lovelace, 2 replies” or “note by ' +
  'Charles Babbage” — and carries its position as “4 of 213” even though only a dozen exist in the ' +
  'DOM. A reply inside a thread is an article of its own. Resolving, expanding and replying are ' +
  'announced politely.';
