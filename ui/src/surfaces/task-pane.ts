/**
 * `<mjx-task-pane>` — **docked, resizable, persistent**, and the one surface in the catalogue a
 * person cannot dismiss.
 *
 * ```html
 * <div class="workspace">
 *   <div class="document">…</div>
 *   <mjx-task-pane label="Format Shape" open dock="inlineEnd" document-color="<the page">
 *     <mjx-checkbox label="Lock aspect ratio"></mjx-checkbox>
 *   </mjx-task-pane>
 * </div>
 * ```
 *
 * ## Three things it does not have, and why each absence is deliberate
 *
 * MJXOFF-188: *"A task pane is docked, resizable and persistent — it is the one surface that is not
 * dismissible, and a gate that treats it like the others will assert the wrong thing."*
 *
 * * **No `SurfaceSession`.** There is no invoker to return focus to, nothing to stack on and
 *   nothing to hold inert. Giving it a session so that all three components looked alike would have
 *   given it an Escape handler, and an Escape handler is exactly the wrong thing.
 * * **No scrim, no top layer, no placement.** It is *in flow*: the document lays out beside it,
 *   which is the whole point of a dock and is why `pinFloating` is not called here. At a phone
 *   width it takes the frame's whole width rather than becoming a sheet, because a sheet is
 *   dismissible and this is not.
 * * **No `dismissals`.** `surfaceKinds.taskPane.dismissals` is empty and every path through this
 *   file goes through `dismissible()`, so there is no branch that could accidentally close it.
 *
 * ## The splitter is a `separator` with a value
 *
 * ARIA's window-splitter pattern, which is the one thing here a keyboard user needs and a pointer
 * user never sees: `role="separator"`, focusable, `aria-valuenow` as a percentage of the boundary,
 * and arrow keys, `Home` and `End` that move it. A drag handle that only a pointer can move is a
 * resizable pane only some people can resize.
 *
 * ## The edge against the document is measured, not chosen
 *
 * The splitter's line is the boundary between this pane and **the user's document**, whose page may
 * be any colour at all — so it is an indicator over a surface we do not control, and it is chosen
 * per document colour by `chooseDockEdgeAmong` rather than fixed. That is U08's rule applied to a
 * second site, and `tests/surfaces.test.ts` sweeps 4,352 document colours through it and asserts
 * the worst case, exactly as U08 does for a swatch.
 *
 * `document-color` is an attribute because the page's colour belongs to the document rather than to
 * us: a shell that knows what the page is fills it in, and a pane that measured against
 * `--document-page` instead would choose correctly for a white page and wrongly for the one on
 * screen.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import { physicalSide, type Direction, type LogicalSide } from '../overlay/floating.ts';
import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import { parseHexColor } from '../tokens/contrast.ts';
import { customPropertyCase, type ThemeMember } from '../tokens/resolver.ts';
import {
  chooseDockEdgeAmong,
  clampFraction,
  dockEdgeCandidates,
  fractionFromDrag,
  fractionFromKey,
  taskPaneCss,
  splitterLabel,
  surfaceEvents,
  surfaceKinds,
  surfaceMotionClass,
  surfacePresentationProperty,
  surfaceProperties,
  surfaceTags,
  surfaceTypeRoles,
  taskPaneDefaultFraction,
  taskPaneFractionBounds,
  type ChosenIndicator,
} from './surface-model.ts';

const styles = taskPaneCss;

let nextTaskPaneSerial = 0;

export class MjxTaskPane extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'open',
    'dock',
    'fraction',
    'document-color',
  ];

  #root: ShadowRoot | undefined;
  #splitter: HTMLElement | undefined;
  #title: HTMLElement | undefined;
  #fraction = taskPaneDefaultFraction;
  #dragging: number | undefined;
  #titleId = '';
  #paneId = '';
  #edge: ChosenIndicator | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.render();
  }

  attributeChangedCallback(name: string): void {
    if (name === 'fraction') {
      const declared = Number.parseFloat(this.getAttribute('fraction') ?? '');
      if (Number.isFinite(declared)) this.#fraction = clampFraction(declared);
    }
    if (this.#root !== undefined) this.render();
  }

  /** The pane's accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** Whether the application is showing it. There is no other way for it to close. */
  get open(): boolean {
    return this.hasAttribute('open');
  }

  set open(value: boolean) {
    if (value) this.setAttribute('open', '');
    else this.removeAttribute('open');
  }

  /** Which edge it docks to, logically. */
  get dock(): LogicalSide {
    const declared = this.getAttribute('dock');
    return declared === 'inlineStart' ? 'inlineStart' : 'inlineEnd';
  }

  /** Its share of the boundary. */
  get fraction(): number {
    return this.#fraction;
  }

  set fraction(value: number) {
    this.#setFraction(clampFraction(value));
  }

  /** What the shell says the document beside the pane is painted in. */
  get documentColor(): string | undefined {
    const declared = this.getAttribute('document-color');
    if (declared === null) return undefined;
    return parseHexColor(declared) === undefined ? undefined : declared;
  }

  /** The edge the pane chose against that document, and how well it reads. */
  get edge(): ChosenIndicator | undefined {
    return this.#edge;
  }

  /**
   * **Which presentation CSS put this pane in** — read back, never computed here.
   *
   * `docked` beside the document, or `full` at a phone width where two columns do not fit.
   */
  get presentation(): 'docked' | 'full' {
    const value = getComputedStyle(this).getPropertyValue(surfacePresentationProperty).trim();
    return value === 'full' ? 'full' : 'docked';
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, styles);
    defineIcon();

    nextTaskPaneSerial += 1;
    this.#titleId = `mjx-task-pane-title-${String(nextTaskPaneSerial)}`;
    this.#paneId = `mjx-task-pane-${String(nextTaskPaneSerial)}`;

    const dock = document.createElement('div');
    dock.className = 'dock';

    const splitter = document.createElement('div');
    splitter.className = 'splitter';
    splitter.setAttribute('part', 'splitter');
    splitter.setAttribute('role', 'separator');
    splitter.setAttribute('aria-orientation', 'vertical');
    splitter.tabIndex = 0;
    splitter.addEventListener('keydown', this.#onSplitterKey);
    splitter.addEventListener('pointerdown', this.#onSplitterDown);
    splitter.addEventListener('pointermove', this.#onSplitterMove);
    splitter.addEventListener('pointerup', this.#onSplitterUp);
    splitter.addEventListener('pointercancel', this.#onSplitterUp);
    this.#splitter = splitter;

    const pane = document.createElement('div');
    pane.className = `pane ${surfaceMotionClass('taskPane')}`;
    pane.setAttribute('part', 'pane');
    // ⚠ `complementary`, not `dialog`. A task pane is a region of the application beside the
    // document, not something drawn over it, and a `dialog` role would tell a screen reader that
    // the document behind it is unavailable — which is the one thing that is never true here.
    pane.setAttribute('role', 'complementary');
    pane.setAttribute('aria-labelledby', this.#titleId);
    pane.id = this.#paneId;

    const header = document.createElement('div');
    header.className = 'header';
    header.setAttribute('part', 'header');

    const title = document.createElement('h2');
    title.className = `title ${surfaceTypeRoles.title}`;
    title.id = this.#titleId;
    title.setAttribute('part', 'title');
    this.#title = title;
    header.append(title);

    const body = document.createElement('div');
    body.className = `body ${surfaceTypeRoles.body}`;
    body.setAttribute('part', 'body');
    body.append(document.createElement('slot'));

    pane.append(header, body);
    dock.append(splitter, pane);
    root.append(dock);
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  render(): void {
    const splitter = this.#splitter;
    if (splitter === undefined) return;

    if (this.#title !== undefined) this.#title.textContent = this.label;

    this.style.setProperty(surfaceProperties.dockFraction, String(this.#fraction));
    splitter.setAttribute('aria-label', splitterLabel(this.label));
    splitter.setAttribute('aria-valuemin', String(Math.round(taskPaneFractionBounds.min * 100)));
    splitter.setAttribute('aria-valuemax', String(Math.round(taskPaneFractionBounds.max * 100)));
    splitter.setAttribute('aria-valuenow', String(Math.round(this.#fraction * 100)));
    // ⚠ The IDREF resolves because both boxes are in **this** shadow root. U04's finding is the
    // one to remember: an IDREF does not cross a shadow boundary, so the same attribute pointing at
    // a slotted element would silently name nothing.
    splitter.setAttribute('aria-controls', this.#paneId);

    this.#syncEdge();
  }

  /**
   * Choose the splitter's line against the document, and write it.
   *
   * When the shell has not said what the document is, nothing is written and the stylesheet falls
   * back to the modal edge — which is the right fallback rather than a guess, because it is the
   * one colour that has already been measured against something.
   */
  #syncEdge(): void {
    const splitter = this.#splitter;
    if (splitter === undefined) return;
    const documentColor = this.documentColor;
    if (documentColor === undefined) {
      this.#edge = undefined;
      this.style.removeProperty(surfaceProperties.dockEdge);
      return;
    }
    const scheme = this.#scheme();
    const chosen = chooseDockEdgeAmong(documentColor, (member) => this.#resolveMember(member, scheme));
    this.#edge = chosen;
    this.style.setProperty(
      surfaceProperties.dockEdge,
      `var(--theme-${customPropertyCase(chosen.member)})`,
    );
  }

  /**
   * What a member resolves to **here**, off the host, falling back to the generated table.
   *
   * The argument-rather-than-lookup rule U08 states: *"a host that has re-themed the platform by
   * declaring `--theme-text-primary` has changed the answer,"* and a component that measured
   * against the generated table would choose correctly for our palette and wrongly for the one on
   * screen.
   */
  #resolveMember(member: ThemeMember, scheme: ColorScheme): string {
    const view = this.ownerDocument.defaultView;
    if (view !== null) {
      const value = view
        .getComputedStyle(this)
        .getPropertyValue(`--theme-${customPropertyCase(member)}`)
        .trim();
      if (parseHexColor(value) !== undefined) return value;
    }
    return tokens.theme[scheme][member as keyof (typeof tokens.theme)['light']];
  }

  #scheme(): ColorScheme {
    const view = this.ownerDocument.defaultView;
    if (view === null) return 'light';
    const declared = this.ownerDocument.documentElement.getAttribute('data-theme');
    if (declared === 'dark') return 'dark';
    if (declared === 'light') return 'light';
    return view.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  // ── resizing ───────────────────────────────────────────────────────────────

  #physicalSide(): 'left' | 'right' {
    const view = this.ownerDocument.defaultView;
    const direction: Direction =
      view !== null && view.getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr';
    const side = physicalSide(this.dock, direction);
    return side === 'left' ? 'left' : 'right';
  }

  #setFraction(value: number): void {
    if (value === this.#fraction) return;
    this.#fraction = value;
    this.render();
    this.dispatchEvent(
      new CustomEvent(surfaceEvents.resize, {
        bubbles: true,
        composed: true,
        detail: { fraction: value, label: this.label },
      }),
    );
  }

  #onSplitterKey = (event: KeyboardEvent): void => {
    const next = fractionFromKey(event.key, this.#fraction, this.#physicalSide());
    if (next === undefined) return;
    event.preventDefault();
    this.#setFraction(next);
  };

  #onSplitterDown = (event: PointerEvent): void => {
    const splitter = this.#splitter;
    if (splitter === undefined) return;
    event.preventDefault();
    splitter.setPointerCapture(event.pointerId);
    this.#dragging = event.pointerId;
  };

  #onSplitterMove = (event: PointerEvent): void => {
    if (this.#dragging !== event.pointerId) return;
    const parent = this.parentElement;
    if (parent === null) return;
    const box = parent.getBoundingClientRect();
    this.#setFraction(
      fractionFromDrag(event.clientX, { start: box.left, size: box.width }, this.#physicalSide()),
    );
  };

  #onSplitterUp = (event: PointerEvent): void => {
    if (this.#dragging !== event.pointerId) return;
    this.#splitter?.releasePointerCapture(event.pointerId);
    this.#dragging = undefined;
  };
}

/** Register the element. Idempotent. */
export function defineTaskPane(): void {
  if (customElements.get(surfaceTags.taskPane) === undefined) {
    customElements.define(surfaceTags.taskPane, MjxTaskPane);
  }
}

/** Re-exported so a gate can name the candidates the pane chooses among. */
export { dockEdgeCandidates, surfaceKinds };

declare global {
  interface HTMLElementTagNameMap {
    'mjx-task-pane': MjxTaskPane;
  }
}
