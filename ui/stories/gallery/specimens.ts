/**
 * The gallery specimens — and, more than in any other story file, **the wiring that makes live
 * preview visible at all.**
 *
 * A gallery with the preview protocol unwired renders identically to one that works, in every
 * screenshot, forever. So this file contains the one thing the catalogue has that a picture does
 * not: a **document to preview into**, and a listener that obeys the protocol. The listener is
 * `applyPreviewEvent` from `src/gallery/preview-session.ts` — the shipped one, not a copy — for the
 * reason `mjx-paint` refuses to compare a painter with itself: a listener written twice, slightly
 * differently, would make a green browser gate evidence about this file rather than about the
 * component.
 *
 * ## Every item's content is static markup with no bindings in it
 *
 * `<mjx-gallery-item>` **captures its children into a fragment** on connect, which means lit's
 * template parts must not point at anything inside one. Attributes on the item element itself are
 * fine — they stay where lit put them — so the varying part of an item (its label, its value, its
 * category) is an attribute, and the art is one of a small set of static templates chosen by index.
 * That is a real constraint of the design and it is written down here rather than discovered by the
 * next child.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import {
  controlStateSpecs,
  type ControlState,
} from '../../src/controls/control-states.ts';
import {
  galleryCellStateNames,
  galleryCellStates,
  galleryItemKindNames,
  galleryItemKinds,
  type GalleryCellState,
} from '../../src/gallery/gallery-model.ts';
import { applyPreviewEvent } from '../../src/gallery/preview-session.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';

// ── the ids the gates open by ────────────────────────────────────────────────

/** Every id a browser gate looks up, named once so a test cannot mistype one. */
export const galleryTestIds = {
  gallery: 'gallery-under-test',
  target: 'preview-target',
  log: 'preview-log',
  stage: 'preview-stage',
  large: 'large-gallery',
  outside: 'outside-the-gallery',
} as const;

/** The attribute the preview target carries the applied value in. */
export const appliedAttribute = 'data-applied';

// ── a note above a specimen ──────────────────────────────────────────────────

export function note(text: string): TemplateResult {
  return html`
    <p
      class=${typeRoleClass('body')}
      style="margin:0;padding:calc(var(--mjx-density-gutter) * 2);color:var(--theme-text-primary);max-inline-size:70ch"
    >
      ${text}
    </p>
  `;
}

const stage = (height: string) =>
  `position:relative;block-size:${height};padding:calc(var(--mjx-density-gutter) * 2);` +
  'border:1px solid var(--theme-border-subtle);border-radius:var(--radius-card);' +
  'margin:calc(var(--mjx-density-gutter) * 2)';

// ── the three item kinds ─────────────────────────────────────────────────────

/**
 * The style-gallery item: a miniature of formatted text.
 *
 * There are eight of them and they are eight *separate static templates* rather than one template
 * with the size interpolated, for the reason in the module note. Eight is what a Styles gallery
 * shows; the shapes below are what a Shapes gallery shows; the swatches are a theme gallery. Three
 * genuinely different kinds, which is what MJXOFF-185 asks the catalogue to prove.
 */
const miniatures: readonly TemplateResult[] = [
  html`<span style="font-size:1.35em;font-weight:800;letter-spacing:-0.02em">AaBbCc</span>`,
  html`<span style="font-size:1.2em;font-weight:700">AaBbCc</span>`,
  html`<span style="font-size:1.05em;font-weight:600">AaBbCc</span>`,
  html`<span style="font-size:1em;font-weight:500">AaBbCc</span>`,
  html`<span style="font-size:1em;font-style:italic;font-weight:400">AaBbCc</span>`,
  html`<span style="font-size:1em;text-decoration:underline">AaBbCc</span>`,
  html`<span style="font-size:0.95em;font-family:var(--font-serif);font-weight:700">AaBbCc</span>`,
  html`<span style="font-size:0.8em;letter-spacing:0.1em;text-transform:uppercase">AaBbCc</span>`,
];

/** The shape-gallery item: inline SVG, taking its colour from the cell's own `currentColor`. */
const shapes: readonly TemplateResult[] = [
  html`<svg viewBox="0 0 40 40" width="36" height="36" aria-hidden="true">
    <rect x="4" y="8" width="32" height="24" rx="3" fill="currentColor" opacity="0.85" />
  </svg>`,
  html`<svg viewBox="0 0 40 40" width="36" height="36" aria-hidden="true">
    <circle cx="20" cy="20" r="15" fill="currentColor" opacity="0.85" />
  </svg>`,
  html`<svg viewBox="0 0 40 40" width="36" height="36" aria-hidden="true">
    <polygon points="20,5 35,33 5,33" fill="currentColor" opacity="0.85" />
  </svg>`,
  html`<svg viewBox="0 0 40 40" width="36" height="36" aria-hidden="true">
    <polygon points="20,4 24,16 36,16 26,24 30,36 20,28 10,36 14,24 4,16 16,16"
      fill="currentColor" opacity="0.85" />
  </svg>`,
  html`<svg viewBox="0 0 40 40" width="36" height="36" aria-hidden="true">
    <path d="M4 30 L14 14 L24 24 L36 8" stroke="currentColor" stroke-width="3" fill="none" />
  </svg>`,
  html`<svg viewBox="0 0 40 40" width="36" height="36" aria-hidden="true">
    <path d="M6 10 h28 v16 h-16 l-8 8 v-8 h-4 z" fill="currentColor" opacity="0.85" />
  </svg>`,
];

/**
 * The colour-gallery item: a block of colour and nothing else.
 *
 * Six static variants over the palette's own members, chosen by index. Every one is a token — a
 * story may write a literal and this one does not need to, and a swatch gallery painted in hexes
 * would be the one specimen that did not move when the palette was re-seeded.
 *
 * ⚠ **Styled inline, and it has to be.** An item's art is cloned into the gallery's **shadow root**,
 * where a document stylesheet does not reach: a `class="swatch"` with a rule in the story's own
 * `<style>` produced eight cells with captions and no colour in them at all, and every gate that
 * counted cells or read a caption stayed green. Custom properties *do* cross the boundary, which is
 * why the values are still tokens. `gallery-item.ts` states the contract and
 * `tests/browser/gallery.spec.ts` asserts a swatch actually paints, so the next author finds out
 * from a failure rather than from a screenshot.
 */
const swatches: readonly TemplateResult[] = [
  html`<span style="display:block;inline-size:100%;block-size:100%;min-block-size:28px;border-radius:var(--radius-chip);border:1px solid var(--theme-border-subtle);background:var(--theme-accent)"></span>`,
  html`<span style="display:block;inline-size:100%;block-size:100%;min-block-size:28px;border-radius:var(--radius-chip);border:1px solid var(--theme-border-subtle);background:var(--theme-accent-border)"></span>`,
  html`<span style="display:block;inline-size:100%;block-size:100%;min-block-size:28px;border-radius:var(--radius-chip);border:1px solid var(--theme-border-subtle);background:var(--theme-secondary-accent)"></span>`,
  html`<span style="display:block;inline-size:100%;block-size:100%;min-block-size:28px;border-radius:var(--radius-chip);border:1px solid var(--theme-border-subtle);background:var(--theme-border)"></span>`,
  html`<span style="display:block;inline-size:100%;block-size:100%;min-block-size:28px;border-radius:var(--radius-chip);border:1px solid var(--theme-border-subtle);background:var(--theme-text-secondary)"></span>`,
  html`<span style="display:block;inline-size:100%;block-size:100%;min-block-size:28px;border-radius:var(--radius-chip);border:1px solid var(--theme-border-subtle);background:var(--theme-surface-raised)"></span>`,
];

/**
 * The rule the swatch variants and the preview target need.
 *
 * ⚠ **Every selector in here is written out literally, and it has to be.** lit does not support
 * bindings inside a `<style>` element — the marker comments it would need are not valid CSS — so
 * `galleryTestIds` above cannot be interpolated into this block. `tests/gallery.test.ts` reads this
 * file and asserts that the ids in it and the ids in `galleryTestIds` are the same set, which turns
 * a limitation of the templating library into a checked fact rather than a comment nobody re-reads.
 *
 * A story may write a length or a weight; `src/` may not. That asymmetry is the lint rule's own
 * (`mjx/no-literal-design-values` is scoped to `src/`) and it is what lets the preview target's
 * eight styles be *genuinely* different from each other in properties a computed-style comparison
 * can read.
 */
export const specimenCss = html`
  <style>
    #preview-target {
      margin: 0;
      padding: calc(var(--mjx-density-gutter) * 2);
      border: 1px solid var(--theme-border-subtle);
      border-radius: var(--radius-card);
      background: var(--theme-surface);
      color: var(--theme-text-primary);
      font-size: 1rem;
      font-weight: 400;
      letter-spacing: normal;
      font-style: normal;
      text-decoration-line: none;
    }
    /* One rule per style, so the *document* genuinely changes and a computed-style comparison has
     * something to compare. Every property here is one the gate reads. */
    #preview-target[data-applied='title'] { font-size: 2rem; font-weight: 800; letter-spacing: -0.03em; }
    #preview-target[data-applied='heading-1'] { font-size: 1.6rem; font-weight: 700; letter-spacing: -0.02em; }
    #preview-target[data-applied='heading-2'] { font-size: 1.35rem; font-weight: 650; letter-spacing: -0.01em; }
    #preview-target[data-applied='normal'] { font-size: 1rem; font-weight: 400; }
    #preview-target[data-applied='quote'] { font-size: 1.1rem; font-style: italic; font-weight: 300; }
    #preview-target[data-applied='emphasis'] { font-size: 1rem; font-weight: 600; text-decoration-line: underline; }
    #preview-target[data-applied='book-title'] { font-size: 1.2rem; font-family: var(--font-serif); font-weight: 700; }
    #preview-target[data-applied='caption'] { font-size: 0.8rem; letter-spacing: 0.14em; }
  </style>
`;

// ── the style gallery, which is the one the preview gate drives ──────────────

interface StyleSpec {
  readonly value: string;
  readonly label: string;
  readonly category: string;
  readonly miniature: number;
  readonly unavailable?: boolean;
  readonly explanation?: string;
}

/**
 * The Styles gallery, in the shape Word's actually has: two categories, eight items, one of them
 * unavailable-and-explained.
 *
 * **Eight is deliberately not a multiple of the column count at any audited width.** A ragged last
 * row is where two-dimensional navigation goes wrong — `ArrowDown` from the penultimate row into a
 * column that does not exist — and a fixture with an exact multiple in it is a fixture that hides
 * the case.
 */
export const styleItems: readonly StyleSpec[] = [
  { value: 'normal', label: 'Normal', category: 'Built-In', miniature: 3 },
  { value: 'title', label: 'Title', category: 'Built-In', miniature: 0 },
  { value: 'heading-1', label: 'Heading 1', category: 'Built-In', miniature: 1 },
  { value: 'heading-2', label: 'Heading 2', category: 'Built-In', miniature: 2 },
  { value: 'quote', label: 'Quote', category: 'Built-In', miniature: 4 },
  { value: 'emphasis', label: 'Emphasis', category: 'Built-In', miniature: 5 },
  { value: 'book-title', label: 'Book Title', category: 'Custom', miniature: 6 },
  {
    value: 'caption',
    label: 'Caption',
    category: 'Custom',
    miniature: 7,
    unavailable: true,
    explanation: 'This document has no caption style defined in its template.',
  },
];

/** The values, in order, so a gate can name what it expects without restating the table. */
export const styleValues: readonly string[] = styleItems.map((item) => item.value);

/** The one item that refuses to preview. */
export const unavailableStyleValue = 'caption';

function styleItem(spec: StyleSpec): TemplateResult {
  return html`<mjx-gallery-item
    value=${spec.value}
    label=${spec.label}
    category=${spec.category}
    ?unavailable=${spec.unavailable === true}
    explanation=${spec.explanation ?? ''}
    >${miniatures[spec.miniature] ?? miniatures[3]}</mjx-gallery-item
  >`;
}

/** The Styles gallery's items. */
export function styleGalleryItems(): TemplateResult[] {
  return styleItems.map(styleItem);
}

// ── the large gallery, which is the one the node-count gate drives ───────────

/**
 * How many items the virtualisation gate uses.
 *
 * MJXOFF-185's brief is explicit that *"a twenty-item fixture proves nothing"*, and this is the
 * number that makes the assertion mean something: four hundred cells is far more than any window,
 * so a gallery that built them all would be caught by a node count rather than by an eye. Office's
 * own theme gallery is of this order once a template pack is installed.
 */
export const largeGalleryItemCount = 400;

/** The categories the large gallery is divided into, so the flyout has real sections to plan. */
export const largeGalleryCategories = ['Office', 'Colourful', 'Monochrome', 'Custom'] as const;

/** The large gallery's items: four hundred swatches, in four sections. */
export function largeGalleryItems(): TemplateResult[] {
  const items: TemplateResult[] = [];
  for (let index = 0; index < largeGalleryItemCount; index += 1) {
    const category =
      largeGalleryCategories[
        Math.floor((index / largeGalleryItemCount) * largeGalleryCategories.length)
      ] ?? largeGalleryCategories[0];
    items.push(
      html`<mjx-gallery-item
        value=${`theme-${String(index)}`}
        label=${`Theme ${String(index + 1)}`}
        category=${category}
        >${swatches[index % swatches.length] ?? swatches[0]}</mjx-gallery-item
      >`,
    );
  }
  return items;
}

// ── the three-kinds specimen ─────────────────────────────────────────────────

/** One gallery per kind, so the *slot* claim is a thing an auditor can see side by side. */
export function threeKindsSpecimen(): TemplateResult {
  return html`
    ${specimenCss}
    <div
      style="display:grid;grid-template-columns:repeat(auto-fit,minmax(20rem,1fr));gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2)"
    >
      ${galleryItemKindNames.map(
        (kind) => html`
          <div data-item-kind=${kind} style="display:grid;gap:var(--mjx-density-step)">
            <p class=${typeRoleClass('label')} style="margin:0">${kind}</p>
            <mjx-gallery label=${kind}>${kindItems(kind)}</mjx-gallery>
            <p
              class=${typeRoleClass('dense')}
              style="margin:0;color:var(--theme-text-secondary);max-inline-size:40ch"
            >
              ${galleryItemKinds[kind].description}
            </p>
          </div>
        `,
      )}
    </div>
  `;
}

function kindItems(kind: (typeof galleryItemKindNames)[number]): TemplateResult[] {
  if (kind === 'textMiniature') return styleItems.slice(0, 6).map(styleItem);
  if (kind === 'shape') {
    const names = ['Rectangle', 'Ellipse', 'Triangle', 'Star', 'Line', 'Callout'];
    return names.map(
      (name, index) => html`<mjx-gallery-item value=${name.toLowerCase()} label=${name}
        >${shapes[index] ?? shapes[0]}</mjx-gallery-item
      >`,
    );
  }
  const names = ['Accent', 'Accent Edge', 'Honey', 'Rule', 'Ink', 'Paper'];
  return names.map(
    (name, index) => html`<mjx-gallery-item value=${`swatch-${String(index)}`} label=${name}
      >${swatches[index] ?? swatches[0]}</mjx-gallery-item
    >`,
  );
}

// ── the live-preview stage ───────────────────────────────────────────────────

/**
 * The document the gallery previews into, and the listener that obeys the protocol.
 *
 * Three lines of listener, and they are the *shipped* three — `applyPreviewEvent`. The stage also
 * writes every event into `#preview-log`, one per line, so the browser gate asserts the **sequence**
 * rather than only the end state: MJXOFF-185 requires *"the live-preview protocol asserted as an
 * event sequence"*, and a gate that read only the document would pass on a component that emitted
 * three cancels for one preview.
 */
export function previewStage(gallery: TemplateResult, height = '18rem'): TemplateResult {
  const state: { applied: string | undefined } = { applied: 'normal' };

  const record = (event: Event, kind: 'preview' | 'cancel' | 'commit'): void => {
    const detail = (event as CustomEvent).detail as {
      value: string;
      label: string;
      by: string;
      restore: string | null;
    };
    applyPreviewEvent(state, {
      kind,
      value: detail.value,
      label: detail.label,
      by: detail.by as 'pointer' | 'keyboard' | 'touch',
      restore: detail.restore ?? undefined,
    });
    const host = event.currentTarget;
    if (!(host instanceof HTMLElement)) return;
    const target = host.querySelector(`#${galleryTestIds.target}`);
    if (target instanceof HTMLElement) target.setAttribute(appliedAttribute, state.applied ?? '');
    const log = host.querySelector(`#${galleryTestIds.log}`);
    if (log instanceof HTMLElement) {
      log.textContent = `${log.textContent ?? ''}${kind}:${detail.value}:${detail.by}:${detail.restore ?? '-'}\n`;
    }
  };

  return html`
    ${specimenCss}
    <div
      id="preview-stage"
      style=${stage(height)}
      @mjx-gallery-preview=${(event: Event) => {
        record(event, 'preview');
      }}
      @mjx-gallery-preview-cancel=${(event: Event) => {
        record(event, 'cancel');
      }}
      @mjx-gallery-commit=${(event: Event) => {
        record(event, 'commit');
      }}
    >
      ${gallery}
      <p id="preview-target" data-applied="normal" style="margin-block-start:calc(var(--mjx-density-gutter) * 2)">
        The quick brown fox jumps over the lazy dog. Hover a style above and watch this paragraph
        take it; move away and watch it come back exactly.
      </p>
      <pre id="preview-log" hidden></pre>
      <button
        id="outside-the-gallery"
        type="button"
        class="mjx-type-control mjx-hit-target"
        style="margin-block-start:var(--mjx-density-step);border:1px solid var(--theme-border);border-radius:var(--radius-control);background:var(--theme-surface);color:var(--theme-text-primary);padding-inline:var(--mjx-density-gutter);cursor:pointer"
      >
        Somewhere else to click
      </button>
    </div>
  `;
}

// ── the states matrix ────────────────────────────────────────────────────────

/**
 * One **inline strip** per state, each holding one cell.
 *
 * One gallery per cell rather than one gallery of seven cells, and the reason is the `focus` cell:
 * a gallery holds **one** roving tab stop, so seven cells in one would be one tab stop and `Tab`
 * could never reach the sixth. Seven galleries are seven tab stops, and the gate walks them with the
 * browser's own sequential navigation exactly as `controls.spec.ts` and `menus.spec.ts` do.
 */
export function galleryStatesMatrix(): TemplateResult {
  return html`
    ${specimenCss}
    <div
      style="display:grid;grid-template-columns:repeat(auto-fit,minmax(12rem,1fr));gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2);align-items:start"
    >
      ${galleryCellStateNames.map(
        (state) => html`
          <div
            data-gallery-state-cell=${state}
            title=${galleryCellStates[state].description}
            style="display:grid;gap:var(--mjx-density-step);justify-items:stretch"
          >
            ${stateSpecimen(state)}
            <p
              class=${typeRoleClass('dense')}
              style="color:var(--theme-text-secondary);margin:0;text-align:center"
            >
              ${state}
            </p>
          </div>
        `,
      )}
    </div>
  `;
}

function stateSpecimen(state: GalleryCellState): TemplateResult {
  switch (state) {
    case 'hover':
      return html`<mjx-gallery label="hover specimen" force-cell-state="hover"
        >${styleItem(styleItems[1] as StyleSpec)}</mjx-gallery
      >`;
    case 'active':
      return html`<mjx-gallery label="active specimen" force-cell-state="active"
        >${styleItem(styleItems[1] as StyleSpec)}</mjx-gallery
      >`;
    case 'selected':
      return html`<mjx-gallery label="selected specimen" value="title"
        >${styleItem(styleItems[1] as StyleSpec)}</mjx-gallery
      >`;
    case 'selectedHover':
      return html`<mjx-gallery label="selected hover specimen" value="title" force-cell-state="hover"
        >${styleItem(styleItems[1] as StyleSpec)}</mjx-gallery
      >`;
    case 'unavailable':
      return html`<mjx-gallery label="unavailable specimen"
        >${styleItem(styleItems[7] as StyleSpec)}</mjx-gallery
      >`;
    // `focus` is never forced: `:focus-visible` is the browser's own judgement about how focus
    // arrived, and a story that asserted it into existence would be auditing a picture of a ring.
    case 'rest':
    case 'focus':
      return html`<mjx-gallery label="${state} specimen"
        >${styleItem(styleItems[1] as StyleSpec)}</mjx-gallery
      >`;
  }
}

/** The states matrix as the story conventions want it. */
export function galleryStatesFor(): readonly { name: string; description: string }[] {
  return galleryCellStateNames.map((state) => ({
    name: state,
    description: galleryCellStates[state].description,
  }));
}

// ── the conventions ──────────────────────────────────────────────────────────

const sharedTokenDependencies: readonly TokenPath[] = [
  'duration.transition',
  'ease.spring',
  'ease.outSoft',
  'fontWeight.bold',
  'fontWeight.medium',
  'leading.snug',
  'leading.tight',
  'radius.chip',
  'radius.control',
  'radius.card',
  'radius.panel',
  'shadow.lift',
  'spacing',
  'text.sm',
  'text.xs',
];

/** The blast radius of a token change, **computed from the state table**. */
export function galleryTokenDependencies(): readonly TokenPath[] {
  const members = new Set<string>();
  for (const state of galleryCellStateNames) {
    const spec = controlStateSpecs[galleryCellStates[state].controlState as ControlState];
    for (const paint of [spec.background, spec.borderColor, spec.text, spec.insetRing]) {
      if (paint !== undefined && paint !== 'transparent') members.add(paint);
    }
  }
  // The surface the flyout is drawn on, the rule above its footer, and the section headings.
  members.add('surfaceRaised');
  members.add('surface');
  members.add('border');
  members.add('borderSubtle');
  members.add('textSecondary');
  members.add('accentPressed');
  const paths = [...members].map((member) => `theme.light.${member}` as TokenPath);
  return [...paths, ...sharedTokenDependencies].sort((left, right) => left.localeCompare(right));
}

/** The keyboard model, written out for the audit from the vocabulary the component obeys. */
export const galleryKeyboard = [
  { keys: 'Tab', does: 'Enters the gallery at its one roving tab stop; from inside, leaves it.' },
  {
    keys: 'Arrow Right / Arrow Left',
    does: 'Moves one item along the reading order, wrapping from the end of a row to the start of the next. Mirrored under RTL.',
  },
  {
    keys: 'Arrow Down / Arrow Up',
    does: 'Moves one row. Down clamps to the last item rather than falling off a ragged last row; up stays put at the first row.',
  },
  { keys: 'Home / End', does: 'Jumps to the first or last item in the whole gallery.' },
  { keys: 'Page Down / Page Up', does: 'Moves a viewport of rows at a time.' },
  {
    keys: 'Every movement key',
    does: '**Emits a preview-request for the item it lands on** — immediately, with no settle delay. Live preview is not a pointer feature.',
  },
  { keys: 'Enter / Space', does: 'Commits the focused item, and closes the flyout if it is open.' },
  {
    keys: 'Escape',
    does: 'Cancels the preview, restoring exactly what was applied before it — and closes the flyout if it is open.',
  },
  { keys: 'Alt + Arrow Down', does: 'Opens the expanded flyout from the strip.' },
];

/** What a screen reader announces. */
export const galleryScreenReader =
  'The gallery announces its name and that it is a list box, then the number of items. In the ' +
  'expanded flyout each category is a group and announces its name once on entry. Each cell ' +
  'announces its own name — never the words drawn inside it, which are “AaBbCc” or a colour — and ' +
  'whether it is selected. An unavailable cell announces “dimmed” and reads its reason; it is ' +
  'still reached by the arrow keys. Moving to a cell requests a live preview, so a screen-reader ' +
  'user gets the same feedback a pointer user does, which is the half of live preview that is ' +
  'usually missing.';
