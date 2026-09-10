/**
 * **The parts every shell is assembled from** — and nothing here is a picture of a component.
 *
 * MJXOFF-274's gate is compositional rather than aesthetic, so this file is written under one rule:
 * *if the catalogue ships a component for it, the shell composes that component.* The only markup
 * of its own is layout — boxes, gaps and a fixed stage — and `tests/shell.test.ts` scans this file
 * for anything else.
 *
 * ## What "does not have to be functional" means here
 *
 * Every component's **own** interaction is live, because the ticket is explicit that *"a menu that
 * cannot open cannot be judged"*. What is not here is command dispatch, any binding to a document,
 * the `ShellBridge` and the real canvas — so the document surface is an honestly labelled
 * placeholder that says so on its face, and every command lights up and does nothing.
 *
 * ## The stage is a fixed height, and that is the assertion
 *
 * A real application fills its window, so a status bar that has been pushed off the bottom at
 * tablet width is a composition defect rather than a scroll. Fixing the block size is what turns
 * *"nothing is clipped"* into something a gate can measure: every named surface must fit inside the
 * stage, and `tests/browser/shell.spec.ts` requires exactly that at all nine sizes in both schemes.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { typeRoleClass } from '../../src/foundations/typography.ts';
import type { MjxDialog } from '../../src/surfaces/dialog.ts';
import type { MjxMenu } from '../../src/menus/menu.ts';
import type { ShellApplication, ShellSize } from './shell-model.ts';

/*
 * ⚠ `data-mjx-shell` and `data-mjx-shell-surface` are written out below as attribute *names*, and
 * imported from `shell-model.ts` as `shellRootAttribute` and `shellSurfaceAttribute` by the gates,
 * because lit's `html` interpolates values and never names. The same arrangement — and the same
 * warning — as `stories/menus/specimens.ts`. What keeps the two spellings honest is that the
 * browser gate asserts it found a root for every one of the nine and a non-empty set of surfaces
 * inside each, so a renamed attribute fails with a count rather than sweeping an empty list.
 */

// ── the stage ────────────────────────────────────────────────────────────────

/**
 * How tall a shell is.
 *
 * One number for all nine, and it is honest rather than lazy: a 900-pixel desktop window and an
 * 844-pixel phone screen are within a rem of one another, and what the audit is looking at is how
 * the chrome divides the height it has rather than how much of it there is.
 */
const stageBlockSize = '52rem';

const frameStyle =
  `display:flex;flex-direction:column;block-size:${stageBlockSize};min-block-size:0;` +
  'min-inline-size:0;overflow:hidden;background:var(--theme-background);' +
  'color:var(--theme-text-primary);font-family:var(--font-sans)';

/**
 * The outermost element of one shell.
 *
 * ⚠ `overflow:hidden`, exactly as an application window clips. It does **not** hide the defect the
 * gate looks for: `scrollWidth` still reports content that ran past the box, which is what
 * `tests/browser/shell.spec.ts` reads.
 */
export function shellFrame(
  application: ShellApplication,
  size: ShellSize,
  ...regions: TemplateResult[]
): TemplateResult {
  return html`
    <div
      data-mjx-shell=${application}
      data-mjx-shell-size=${size}
      style=${frameStyle}
    >
      ${regions}
    </div>
  `;
}

/** A named surface: a box the layout gate requires to be present, sized and unclipped. */
export function surface(name: string, style: string, ...content: TemplateResult[]): TemplateResult {
  return html`<div data-mjx-shell-surface=${name} style=${style}>${content}</div>`;
}

// ── the honest placeholder ───────────────────────────────────────────────────

/**
 * ⚠ `overflow:auto` and a tab stop, and **both are required rather than tidy**.
 *
 * A document is taller than its window by definition, so the area it is drawn in has to scroll — and
 * without that the placeholder's own paragraphs set the flex line's height and push the surface
 * three hundred pixels past the foot of the shell at tablet width, which the layout gate reports as
 * a clipped surface. `tabindex="0"` then follows twice over: axe's `scrollable-region-focusable`
 * refuses a scroll container a keyboard cannot reach, and `<mjx-context-menu>` opens *at the
 * keyboard* on Shift + F10, which needs the canvas to be somewhere the keyboard can be.
 */
const canvasStyle =
  'flex:1 1 auto;min-inline-size:0;min-block-size:0;display:flex;flex-direction:column;' +
  'gap:var(--mjx-density-gutter);justify-content:center;' +
  'padding:calc(var(--mjx-density-gutter) * 2);overflow:auto';

/**
 * The style a shell puts on the `<mjx-context-menu>` that owns the document surface.
 *
 * ⚠ **The context menu goes *inside* the canvas area, around the page — not around the canvas
 * area.** It was written the other way first, and the shape of the failure is worth keeping: the
 * component's own shadow root is a `display: block` region with an automatic height, so a canvas
 * area slotted into it has nothing to fill and takes its content's height instead. The placeholder's
 * own paragraphs then set the height of the whole document column, and at tablet width the surface
 * ran 218 px past the foot of the shell. Wrapping the *page* rather than the *area* is also the more
 * honest region: a right-click belongs to the document, not to the padding around it.
 */
export const contextRegionStyle = 'display:block;inline-size:100%';

const pageStyle =
  'inline-size:100%;max-inline-size:44rem;margin-inline:auto;min-block-size:12rem;display:flex;' +
  'flex-direction:column;gap:var(--mjx-density-gutter);align-items:center;justify-content:center;' +
  'text-align:center;padding:calc(var(--mjx-density-gutter) * 3)';

/**
 * **The document surface, standing in for the canvas, and saying so.**
 *
 * MJXOFF-274: *"a placeholder document surface standing in for the canvas, honestly labelled. The
 * real renderer is Rust and is not wired into Storybook in this loop."* The sentence is on the
 * placeholder rather than in a caption because a caption is read once and a shell is looked at for
 * an hour — and the second sentence is the one the ticket asks for by name: a reviewer must not
 * wonder why pressing Bold does nothing.
 */
export function documentPlaceholder(what: string, selection?: TemplateResult): TemplateResult {
  return html`
    <mjx-surface level="raised" radius="card" style=${pageStyle}>
      <p class=${typeRoleClass('paneTitle')} style="margin:0;color:var(--theme-text-primary)">
        ${what}
      </p>
      <p
        class=${typeRoleClass('body')}
        style="margin:0;max-inline-size:48ch;color:var(--theme-text-secondary)"
      >
        A placeholder. The renderer is Rust — <code>mjx-paint</code> through
        <code>mjx-scene</code> — and is not wired into Storybook, so nothing here is a rendered
        document.
      </p>
      <p
        class=${typeRoleClass('dense')}
        style="margin:0;max-inline-size:52ch;color:var(--theme-text-secondary)"
      >
        Every command in this shell is live as a <em>control</em> and inert as a
        <em>command</em>: menus open, galleries preview, panes resize — and nothing changes a
        document, because command dispatch and document binding are loop 2.
      </p>
      ${selection ?? html``}
    </mjx-surface>
  `;
}

/** A run of text a mini toolbar can be anchored to, so a selection is a real box on the page. */
export function selectionRun(id: string, text: string): TemplateResult {
  return html`<mark
    id=${id}
    class=${typeRoleClass('body')}
    style="background:var(--theme-accent-surface);color:var(--theme-text-primary);
           border-radius:var(--radius-chip);padding:0 var(--mjx-density-step)"
    >${text}</mark
  >`;
}

// ── ribbon vocabulary ────────────────────────────────────────────────────────

/** One group of a ribbon tab. */
export function group(
  label: string,
  priority: 'primary' | 'standard' | 'secondary' | 'ancillary',
  options: { readonly launcher?: string },
  ...children: TemplateResult[]
): TemplateResult {
  return html`
    <mjx-ribbon-group label=${label} priority=${priority}>
      ${children}
      ${options.launcher === undefined
        ? html``
        : html`<mjx-dialog-launcher
            slot="dialog-launcher"
            label=${options.launcher}
          ></mjx-dialog-launcher>`}
    </mjx-ribbon-group>
  `;
}

/** A tab whose only content is one small group — the eight tabs nobody opened this ribbon for. */
export function stubTab(id: string, label: string, command: string, icon: string): TemplateResult {
  return html`
    <mjx-ribbon-tab tab-id=${id} label=${label}>
      <mjx-ribbon-group label=${label} priority="standard">
        <mjx-button label=${command} icon=${icon} size="large"></mjx-button>
      </mjx-ribbon-group>
    </mjx-ribbon-tab>
  `;
}

/**
 * **How wide a field is when it is in a ribbon**, and this is a shell decision rather than a
 * component one.
 *
 * A field left to its own devices takes the width its content wants — a font picker asks for about
 * four hundred pixels — and three of them in one group make the group wider than the tab, at which
 * point `.body { flex-wrap: wrap }` puts the next group on a second row. So the assembly states the
 * width, exactly as Office's own ribbon does: a font box is a fixed width there too, and the name is
 * ellipsised rather than the box being grown.
 *
 * ⚠ The wrapping itself is a **finding**, not a thing these numbers fix. See `ui/README.md`: a
 * ribbon that runs out of room wraps rather than demoting a group, because the collapse ladder is
 * driven by the ribbon's own `@container` width and not by how much room is left on the row.
 */
export const ribbonFieldStyle = 'inline-size:11rem;flex:0 0 auto';

/**
 * The same, for the short fields: a size box, a number format.
 *
 * ⚠ **7 rem and not 5.5, and the 1.5 rem is a component's floor rather than a taste.** A
 * `<mjx-dropdown>` given 88 px lays out 112 px of field and overflows it silently — the host shrinks
 * and the field inside does not. Seven rem is the first width at which it does not, and
 * `ui/README.md` records the floor so the next shell does not rediscover it by eye.
 */
export const ribbonNarrowFieldStyle = 'inline-size:7rem;flex:0 0 auto';

/** And for a colour box, which needs room for a swatch and a name. */
export const ribbonColourFieldStyle = 'inline-size:9rem;flex:0 0 auto';

/**
 * **How tall a gallery is when it is in a ribbon**, and this one is a finding wearing a fix.
 *
 * Left alone, `<mjx-gallery>` in a ribbon group asks for **266 px** — which at 1440 is half the
 * ribbon and twice what the row beside it needs. Office's Styles gallery is about a quarter of that
 * and shows one row with a scroll pair, which is exactly what the component does when it is given
 * the height. So the shell gives it one.
 *
 * It is written here rather than sprinkled into three stories because it is one decision about one
 * component, and because it is the number `ui/README.md` names when it says the gallery's intrinsic
 * height in a ribbon is a thing MJXOFF-195 should look at.
 *
 * ⚠ **`overflow:hidden` is part of the number and not tidiness.** Given a height, the gallery lays
 * its cells out past it: the second row of styles rendered *below the ribbon*, over the navigation
 * pane and the document, in every one of the six wide shells. Its own scroll pair is drawn inside
 * the box and works; what does not happen is the clipping. That is a component finding, recorded in
 * `ui/README.md`, and this is the shell holding the line until it is decided.
 */
export const ribbonGalleryStyle = 'block-size:6rem;overflow:hidden';

/** A toggle that starts on, so the ribbon shows a pressed state without a pointer. */
export function toggle(label: string, icon: string, pressed = false): TemplateResult {
  return html`<mjx-toggle-button
    slot="essential"
    label=${label}
    icon=${icon}
    size="icon"
    ?pressed=${pressed}
  ></mjx-toggle-button>`;
}

// ── wiring ───────────────────────────────────────────────────────────────────

/**
 * Open the surface named by `data-opens` on the element that was activated.
 *
 * ⚠ **This is the only wiring in the assembly, and it is deliberately one function.** Every other
 * interaction in these shells belongs to the component that owns it: a popover finds its own
 * anchor, a context menu hears its own right-click, a gallery runs its own preview session, a task
 * pane drags its own splitter. What no component can do for itself is know *which* dialog a
 * particular group's launcher opens, because that is an application decision — so the shell states
 * it as an attribute and this reads it.
 *
 * Command dispatch is out of scope, so nothing else is wired at all.
 */
export function openDeclaredSurface(event: Event): void {
  const source = event.target;
  if (!(source instanceof HTMLElement)) return;
  const trigger = source.closest('[data-opens]');
  if (!(trigger instanceof HTMLElement)) return;
  const id = trigger.dataset['opens'];
  if (id === undefined) return;
  const target = document.getElementById(id);
  if (target === null) return;
  if (target.localName === 'mjx-menu') (target as MjxMenu).openFrom(trigger);
  else (target as MjxDialog).open = true;
}

/** Open a sheet when the phone's command rail reports the command that owns it. */
export function openSheetOnCommand(
  commandId: string,
  sheetId: string,
): (event: Event) => void {
  return (event: Event): void => {
    const detail = (event as CustomEvent<{ id?: string }>).detail;
    if (detail?.id !== commandId) return;
    const sheet = document.getElementById(sheetId);
    if (sheet !== null) (sheet as MjxDialog).open = true;
  };
}

// ── the workspace ────────────────────────────────────────────────────────────

export const workspaceStyle =
  'flex:1 1 auto;display:flex;min-block-size:0;min-inline-size:0;align-items:stretch';

/**
 * A navigator pane whose width is the splitter's fraction.
 *
 * `box-sizing:border-box` and the padding scaled by `--mjx-split-collapsed` are MJXOFF-190's
 * findings rather than taste: a padded region set to zero inline size is still its own padding
 * wide, and CSS floors the used width there under either box model.
 */
export function navigatorPane(id: string, fallbackFraction: string, inner: TemplateResult): TemplateResult {
  const style =
    `flex:0 0 auto;box-sizing:border-box;min-inline-size:0;overflow:hidden;display:flex;` +
    `inline-size:calc(100% * var(--mjx-split-fraction, ${fallbackFraction}));` +
    'padding:calc(var(--mjx-density-gutter) * (1 - var(--mjx-split-collapsed, 0)));' +
    'padding-inline-end:0';
  return html`<div id=${id} style=${style}>${inner}</div>`;
}

/** The divider between a navigator and the document. */
export function paneSplitter(id: string, controls: string, fraction: string): TemplateResult {
  return html`<mjx-splitter
    id=${id}
    label="the navigation pane"
    controls=${controls}
    fraction=${fraction}
    min-start="0.12"
    min-end="0.4"
  ></mjx-splitter>`;
}

/** The column the document lives in: whatever chrome sits above it, the canvas, whatever sits below. */
export function documentColumn(...content: TemplateResult[]): TemplateResult {
  return html`<div
    style="flex:1 1 auto;min-inline-size:0;min-block-size:0;display:flex;flex-direction:column;
           overflow:hidden"
  >
    ${content}
  </div>`;
}

/** The canvas and its scrollbar, side by side, filling what is left. */
export function canvasRow(...content: TemplateResult[]): TemplateResult {
  return html`<div
    style="flex:1 1 auto;min-block-size:0;min-inline-size:0;display:flex;gap:var(--mjx-density-step);
           padding:var(--mjx-density-gutter);padding-inline-end:var(--mjx-density-step)"
  >
    ${content}
  </div>`;
}

/** The area a canvas placeholder is centred in, and a named surface: the document itself. */
export function canvasArea(id: string, ...content: TemplateResult[]): TemplateResult {
  return html`<div
    id=${id}
    data-mjx-shell-surface="document"
    tabindex="0"
    style=${canvasStyle}
  >
    ${content}
  </div>`;
}

// ── panes ────────────────────────────────────────────────────────────────────

/** The inside of a task pane: a stack of fields with the pane's own gutter between them. */
export function paneStack(...content: TemplateResult[]): TemplateResult {
  return html`<div
    style="display:flex;flex-direction:column;gap:var(--mjx-density-gutter);min-inline-size:0"
  >
    ${content}
  </div>`;
}

/** A field with its name above it, the way every pane in Office lays one out. */
export function field(id: string, name: string, control: TemplateResult): TemplateResult {
  return html`<div style="display:grid;gap:var(--mjx-density-step);min-inline-size:0">
    <mjx-label for=${id}>${name}</mjx-label>
    ${control}
  </div>`;
}

/** A quiet heading inside a pane, for the sections a pane divides itself into. */
export function paneHeading(text: string): TemplateResult {
  return html`<p
    class=${typeRoleClass('label')}
    style="margin:calc(var(--mjx-density-gutter) * 2) 0 0;color:var(--theme-text-secondary)"
  >
    ${text}
  </p>`;
}

// ── the status bar ───────────────────────────────────────────────────────────

/** One reading in a status bar. */
export interface ShellSegment {
  readonly id: string;
  readonly label: string;
  readonly value: string;
  readonly priority: 'ancillary' | 'supplementary' | 'standard' | 'essential';
  readonly region?: 'start' | 'centre' | 'end';
}

/**
 * The bar across the foot of a desktop shell.
 *
 * The zoom control goes in the `end` region because that is where Office puts it and because it is
 * the one *control* among a row of readings — which is also why it is the segment a narrowing shell
 * must not demote, and why it is declared `essential`.
 */
export function statusBar(
  id: string,
  label: string,
  segments: readonly ShellSegment[],
  zoom: TemplateResult,
): TemplateResult {
  return surface(
    'status-bar',
    'flex:0 0 auto;min-inline-size:0',
    html`
      <mjx-status-bar id=${id} label=${label}>
        ${segments.map(
          (segment) => html`
            <mjx-status-segment
              id=${segment.id}
              label=${segment.label}
              value=${segment.value}
              priority=${segment.priority}
              region=${segment.region ?? 'start'}
            ></mjx-status-segment>
          `,
        )}
        ${zoom}
      </mjx-status-bar>
    `,
  );
}

/** The zoom control, told what it is looking at so its two fits have real answers. */
export function zoom(id: string, page: { width: number; height: number }): TemplateResult {
  return html`<mjx-zoom-control
    id=${id}
    percent="100"
    viewport-width="900"
    viewport-height="620"
    page-width=${String(page.width)}
    page-height=${String(page.height)}
  ></mjx-zoom-control>`;
}

// ── the phone ────────────────────────────────────────────────────────────────

/**
 * The phone's document area: everything above the rails, and the rails at the block end.
 *
 * The bars are pinned to the bottom because that is where a thumb is — MJXOFF-194 measures it — and
 * the document takes what is left rather than the bars floating over it.
 */
export function phoneBody(...content: TemplateResult[]): TemplateResult {
  return html`<div
    style="flex:1 1 auto;min-block-size:0;display:flex;flex-direction:column;overflow:hidden;
           padding:var(--mjx-density-gutter)"
  >
    ${content}
  </div>`;
}

/** The two rails a phone shell ends with, in the order a thumb reaches them. */
export function phoneRails(...content: TemplateResult[]): TemplateResult {
  return surface(
    'command-rail',
    'flex:0 0 auto;display:flex;flex-direction:column;gap:var(--mjx-density-step);min-inline-size:0',
    ...content,
  );
}
